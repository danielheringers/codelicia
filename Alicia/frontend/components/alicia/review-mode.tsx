"use client"

import {
  Check,
  CheckCheck,
  CheckSquare,
  Clock,
  FileCode2,
  Filter,
  Minus,
  ShieldAlert,
  ShieldCheck,
  Square,
  X,
} from "lucide-react"
import { useCallback, useEffect, useMemo, useRef, useState } from "react"

import { DiffViewer } from "@/components/alicia/diff-viewer"
import { type FileChange } from "@/lib/alicia-types"
import {
  countReviewFindingsByFile,
  extractReviewCommentsByFile,
  parseAgentDiffMarkdownSegments,
  parseDiffSystemMessage,
  parseUnifiedDiffFiles,
  type ApprovalRequestState,
  type DiffFileView,
  type Message,
} from "@/lib/alicia-runtime-helpers"

type FileDecision = "pending" | "reviewed" | "approved" | "rejected"

type FilterMode = "all" | "pending" | "reviewed" | "approved" | "rejected" | "with-findings"
type SortMode = "name" | "status" | "findings"

const decisionConfig: Record<FileDecision, { icon: typeof Clock; color: string; label: string }> = {
  pending:  { icon: Clock, color: "text-muted-foreground", label: "Pending" },
  reviewed: { icon: CheckCheck, color: "text-terminal-blue", label: "Reviewed" },
  approved: { icon: ShieldCheck, color: "text-terminal-green", label: "Approved" },
  rejected: { icon: ShieldAlert, color: "text-terminal-red", label: "Rejected" },
}

const decisionSortOrder: Record<FileDecision, number> = {
  pending: 0,
  reviewed: 1,
  approved: 2,
  rejected: 3,
}

interface ReviewModeProps {
  fileChanges: FileChange[]
  turnDiffFiles: DiffFileView[]
  pendingApprovals: ApprovalRequestState[]
  reviewMessages: Message[]
  isReviewThinking: boolean
  isReviewComplete: boolean
  onRunReview: () => void
  onRunReviewFile: (selectedPath: string) => void
  onCommitApproved: (payload: {
    approvedPaths: string[]
    message: string
    comments: Record<string, string>
  }) => Promise<void>
  onClose: () => void
}

interface ReviewFileItem {
  name: string
  status: FileChange["status"]
  diff: DiffFileView | null
}

type ReviewFeedEntry =
  {
    id: string
    label: string
    files: DiffFileView[]
  }

function inferStatusFromDiff(diff: DiffFileView): FileChange["status"] {
  const additions = diff.lines.some((line) => line.type === "add")
  const removals = diff.lines.some((line) => line.type === "remove")
  if (additions && removals) return "modified"
  if (additions) return "added"
  if (removals) return "deleted"
  return "modified"
}

const statusClass: Record<FileChange["status"], string> = {
  modified: "text-terminal-blue",
  added: "text-terminal-green",
  deleted: "text-terminal-red",
  renamed: "text-terminal-cyan",
  copied: "text-terminal-cyan",
  untracked: "text-terminal-gold",
  unmerged: "text-terminal-red",
}

const statusLabel: Record<FileChange["status"], string> = {
  modified: "M",
  added: "A",
  deleted: "D",
  renamed: "R",
  copied: "C",
  untracked: "?",
  unmerged: "!",
}

export function ReviewMode({
  fileChanges,
  turnDiffFiles,
  pendingApprovals,
  reviewMessages,
  isReviewThinking,
  isReviewComplete,
  onRunReview,
  onRunReviewFile,
  onCommitApproved,
  onClose,
}: ReviewModeProps) {
  const files = useMemo<ReviewFileItem[]>(() => {
    const byName = new Map<string, ReviewFileItem>()

    for (const item of fileChanges) {
      byName.set(item.name, {
        name: item.name,
        status: item.status,
        diff: turnDiffFiles.find((entry) => entry.filename === item.name) ?? null,
      })
    }

    for (const diff of turnDiffFiles) {
      if (byName.has(diff.filename)) {
        continue
      }

      byName.set(diff.filename, {
        name: diff.filename,
        status: inferStatusFromDiff(diff),
        diff,
      })
    }

    return Array.from(byName.values())
  }, [fileChanges, turnDiffFiles])

  const reviewFeedEntries = useMemo<ReviewFeedEntry[]>(() => {
    const entries: ReviewFeedEntry[] = []

    for (const message of reviewMessages) {
      const diffPayload = parseDiffSystemMessage(message.content)
      if (diffPayload) {
        const resolvedFiles =
          diffPayload.version === 1
            ? parseUnifiedDiffFiles(diffPayload.diff)
            : turnDiffFiles

        if (resolvedFiles.length > 0) {
          entries.push({
            id: `message-${message.id}-diff-system`,
            label: diffPayload.title ?? "Review diff update",
            files: resolvedFiles,
          })
        }
        continue
      }

      const parsedSegments = parseAgentDiffMarkdownSegments(message.content)
      const segmentFiles = parsedSegments.flatMap((segment) =>
        segment.kind === "diff" ? segment.files : [],
      )
      if (segmentFiles.length > 0) {
        entries.push({
          id: `message-${message.id}-diff-md`,
          label: message.type === "agent" ? "Alicia diff update" : "Review diff update",
          files: segmentFiles,
        })
        continue
      }

      const fallbackFiles = parseUnifiedDiffFiles(message.content)
      if (fallbackFiles.length > 0) {
        entries.push({
          id: `message-${message.id}-diff-inline`,
          label: "Review diff update",
          files: fallbackFiles,
        })
        continue
      }
    }

    return entries
  }, [reviewMessages, turnDiffFiles])

  const reviewCommentsByFile = useMemo(
    () => extractReviewCommentsByFile(reviewMessages, files.map((file) => file.name)),
    [files, reviewMessages],
  )

  const [selectedPath, setSelectedPath] = useState<string | null>(null)
  const [decisions, setDecisions] = useState<Record<string, FileDecision>>({})
  const [comments, setComments] = useState<Record<string, string>>({})
  const [manualCommentEdits, setManualCommentEdits] = useState<Record<string, boolean>>({})
  const [expandedComments, setExpandedComments] = useState<Record<string, boolean>>({})
  const [commitMessage, setCommitMessage] = useState("chore(review): commit approved files")
  const [isCommitting, setIsCommitting] = useState(false)
  const [hasPendingFeedItems, setHasPendingFeedItems] = useState(0)
  const [feedAtBottom, setFeedAtBottom] = useState(true)
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set())
  const [filterMode, setFilterMode] = useState<FilterMode>("all")
  const [sortMode, setSortMode] = useState<SortMode>("name")
  const [reviewCompletionApplied, setReviewCompletionApplied] = useState(false)
  const [showFilterDropdown, setShowFilterDropdown] = useState(false)

  const findingsCountByFile = useMemo(
    () => countReviewFindingsByFile(reviewCommentsByFile),
    [reviewCommentsByFile],
  )

  const displayFiles = useMemo(() => {
    let filtered = files
    if (filterMode === "pending") {
      filtered = files.filter(f => (decisions[f.name] ?? "pending") === "pending")
    } else if (filterMode === "reviewed") {
      filtered = files.filter(f => decisions[f.name] === "reviewed")
    } else if (filterMode === "approved") {
      filtered = files.filter(f => decisions[f.name] === "approved")
    } else if (filterMode === "rejected") {
      filtered = files.filter(f => decisions[f.name] === "rejected")
    } else if (filterMode === "with-findings") {
      filtered = files.filter(f => (findingsCountByFile[f.name] ?? 0) > 0)
    }

    const sorted = [...filtered]
    if (sortMode === "status") {
      sorted.sort((a, b) => {
        const da = decisionSortOrder[decisions[a.name] ?? "pending"] ?? 0
        const db = decisionSortOrder[decisions[b.name] ?? "pending"] ?? 0
        return da - db || a.name.localeCompare(b.name)
      })
    } else if (sortMode === "findings") {
      sorted.sort((a, b) => {
        const fa = findingsCountByFile[a.name] ?? 0
        const fb = findingsCountByFile[b.name] ?? 0
        return fb - fa || a.name.localeCompare(b.name)
      })
    } else {
      sorted.sort((a, b) => a.name.localeCompare(b.name))
    }

    return sorted
  }, [files, filterMode, sortMode, decisions, findingsCountByFile])

  const statusCounts = useMemo(() => ({
    all: files.length,
    pending: files.filter(f => (decisions[f.name] ?? "pending") === "pending").length,
    reviewed: files.filter(f => decisions[f.name] === "reviewed").length,
    approved: files.filter(f => decisions[f.name] === "approved").length,
    rejected: files.filter(f => decisions[f.name] === "rejected").length,
    "with-findings": files.filter(f => (findingsCountByFile[f.name] ?? 0) > 0).length,
  }), [files, decisions, findingsCountByFile])

  const reviewSummary = useMemo(() => ({
    totalFiles: files.length,
    reviewedCount: files.filter(f => decisions[f.name] === "reviewed").length,
    approvedCount: files.filter(f => decisions[f.name] === "approved").length,
    rejectedCount: files.filter(f => decisions[f.name] === "rejected").length,
    pendingCount: files.filter(f => (decisions[f.name] ?? "pending") === "pending").length,
    totalFindings: Object.values(findingsCountByFile).reduce((sum, count) => sum + count, 0),
  }), [files, decisions, findingsCountByFile])

  const feedRef = useRef<HTMLDivElement | null>(null)
  const previousFeedEntryCountRef = useRef(0)

  const isFeedNearBottom = useCallback((container: HTMLDivElement) => {
    const thresholdPx = 72
    const remaining = container.scrollHeight - container.scrollTop - container.clientHeight
    return remaining <= thresholdPx
  }, [])

  const scrollFeedToBottom = useCallback(() => {
    const container = feedRef.current
    if (!container) {
      return
    }
    container.scrollTop = container.scrollHeight
    setFeedAtBottom(true)
    setHasPendingFeedItems(0)
  }, [])

  useEffect(() => {
    const container = feedRef.current
    if (!container) {
      return
    }

    const onScroll = () => {
      const atBottom = isFeedNearBottom(container)
      setFeedAtBottom(atBottom)
      if (atBottom) {
        setHasPendingFeedItems(0)
      }
    }

    onScroll()
    container.addEventListener("scroll", onScroll, { passive: true })
    return () => {
      container.removeEventListener("scroll", onScroll)
    }
  }, [isFeedNearBottom])

  useEffect(() => {
    const nextCount = reviewFeedEntries.length + (isReviewThinking ? 1 : 0)
    const delta = Math.max(0, nextCount - previousFeedEntryCountRef.current)
    previousFeedEntryCountRef.current = nextCount

    if (delta === 0) {
      return
    }

    const container = feedRef.current
    if (!container) {
      return
    }

    if (feedAtBottom || isFeedNearBottom(container)) {
      window.requestAnimationFrame(() => {
        scrollFeedToBottom()
      })
      return
    }

    setHasPendingFeedItems((previous) => previous + delta)
  }, [feedAtBottom, isFeedNearBottom, isReviewThinking, reviewFeedEntries, scrollFeedToBottom])

  useEffect(() => {
    setDecisions((previous) => {
      const next: Record<string, FileDecision> = {}
      for (const file of files) {
        next[file.name] = previous[file.name] ?? "pending"
      }
      return next
    })

    setComments((previous) => {
      const next: Record<string, string> = {}
      for (const file of files) {
        next[file.name] = previous[file.name] ?? ""
      }
      return next
    })

    setManualCommentEdits((previous) => {
      const next: Record<string, boolean> = {}
      for (const file of files) {
        next[file.name] = previous[file.name] ?? false
      }
      return next
    })

    setExpandedComments((previous) => {
      const next: Record<string, boolean> = {}
      for (const file of files) {
        next[file.name] = previous[file.name] ?? false
      }
      return next
    })

    setSelectedPath((previous) => {
      if (previous && files.some((file) => file.name === previous)) {
        return previous
      }
      return files[0]?.name ?? null
    })

    setSelectedPaths(prev => {
      const validNames = new Set(files.map(f => f.name))
      const filtered = new Set([...prev].filter(p => validNames.has(p)))
      return filtered.size === prev.size ? prev : filtered
    })
  }, [files])

  useEffect(() => {
    setComments((previous) => {
      let changed = false
      const next = { ...previous }

      for (const file of files) {
        const autoComment = reviewCommentsByFile[file.name] ?? ""
        if (manualCommentEdits[file.name]) {
          continue
        }

        if ((next[file.name] ?? "") !== autoComment) {
          next[file.name] = autoComment
          changed = true
        }
      }

      return changed ? next : previous
    })
  }, [files, manualCommentEdits, reviewCommentsByFile])

  useEffect(() => {
    if (!isReviewComplete || reviewCompletionApplied) return
    setDecisions(prev => {
      const next = { ...prev }
      for (const file of files) {
        if (next[file.name] !== "pending") continue
        if (!(reviewCommentsByFile[file.name] ?? "").trim()) {
          next[file.name] = "reviewed"
        }
      }
      return next
    })
    setReviewCompletionApplied(true)
  }, [isReviewComplete, reviewCompletionApplied, files, reviewCommentsByFile])

  useEffect(() => {
    if (isReviewThinking) setReviewCompletionApplied(false)
  }, [isReviewThinking])

  const selectedFile = useMemo(
    () => files.find((file) => file.name === selectedPath) ?? null,
    [files, selectedPath],
  )
  const selectedFileHasConflict = selectedFile?.status === "unmerged"

  const approvedPaths = useMemo(
    () => files
      .filter((file) => decisions[file.name] === "approved" || decisions[file.name] === "reviewed")
      .map((file) => file.name),
    [decisions, files],
  )

  const canCommit = approvedPaths.length > 0 && commitMessage.trim().length > 0 && !isCommitting

  const selectedDiffStats = useMemo(() => {
    if (!selectedFile?.diff) {
      return { additions: 0, removals: 0 }
    }

    const additions = selectedFile.diff.lines.filter((line) => line.type === "add").length
    const removals = selectedFile.diff.lines.filter((line) => line.type === "remove").length
    return { additions, removals }
  }, [selectedFile])

  const toggleFileSelection = useCallback((path: string) => {
    setSelectedPaths(prev => {
      const next = new Set(prev)
      if (next.has(path)) { next.delete(path) } else { next.add(path) }
      return next
    })
  }, [])

  const selectAll = useCallback(
    () => setSelectedPaths(new Set(displayFiles.map(f => f.name))),
    [displayFiles],
  )

  const deselectAll = useCallback(() => setSelectedPaths(new Set()), [])

  const isAllSelected = displayFiles.length > 0 && displayFiles.every(f => selectedPaths.has(f.name))
  const isSomeSelected = displayFiles.some(f => selectedPaths.has(f.name)) && !isAllSelected

  const batchApprove = useCallback(() => {
    setDecisions(prev => {
      const next = { ...prev }
      for (const path of selectedPaths) {
        const file = files.find(f => f.name === path)
        if (file && file.status !== "unmerged") {
          next[path] = "approved"
        }
      }
      return next
    })
    setSelectedPaths(new Set())
  }, [selectedPaths, files])

  const batchReject = useCallback(() => {
    setDecisions(prev => {
      const next = { ...prev }
      for (const path of selectedPaths) {
        next[path] = "rejected"
      }
      return next
    })
    setSelectedPaths(new Set())
  }, [selectedPaths])

  const batchReset = useCallback(() => {
    setDecisions(prev => {
      const next = { ...prev }
      for (const path of selectedPaths) {
        next[path] = "pending"
      }
      return next
    })
    setSelectedPaths(new Set())
  }, [selectedPaths])

  const handleCommitApproved = async () => {
    if (!canCommit) {
      return
    }

    setIsCommitting(true)
    try {
      await onCommitApproved({
        approvedPaths,
        message: commitMessage.trim(),
        comments,
      })
    } finally {
      setIsCommitting(false)
    }
  }

  return (
    <div className="fixed inset-0 z-50 bg-background">
      <div className="h-full w-full bg-panel-bg overflow-hidden flex flex-col">
        <div className="flex items-center justify-between px-4 py-3 border-b border-panel-border">
          <div className="flex items-center gap-2">
            <FileCode2 className="w-4 h-4 text-terminal-blue" />
            <span className="text-sm font-semibold text-terminal-fg">Review Mode</span>
            <span className="text-[10px] text-muted-foreground bg-background/60 px-1.5 py-0.5 rounded">
              {files.length} file(s)
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={onRunReview}
              className="px-2 py-1 rounded text-xs bg-terminal-blue/15 text-terminal-blue hover:bg-terminal-blue/25"
            >
              Run /review
            </button>
            <button
              onClick={() => {
                if (!selectedPath) {
                  return
                }
                onRunReviewFile(selectedPath)
              }}
              disabled={!selectedPath}
              className="px-2 py-1 rounded text-xs bg-terminal-cyan/15 text-terminal-cyan hover:bg-terminal-cyan/25 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Run /review-file
            </button>
            <button
              onClick={onClose}
              className="p-1 rounded hover:bg-[#b9bcc01c] text-muted-foreground"
              aria-label="Close review mode"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>

        {files.length === 0 ? (
          <div className="flex-1 p-6 flex flex-col items-center justify-center gap-3 text-center">
            <FileCode2 className="w-8 h-8 text-muted-foreground/60" />
            <p className="text-sm text-terminal-fg">No file changes available for review.</p>
            <p className="text-xs text-muted-foreground">
              Execute a turn with file updates or run <code>/review</code> again.
            </p>
          </div>
        ) : (
          <div className="flex-1 min-h-0 grid grid-cols-[minmax(240px,320px)_1fr]">
            <div className="border-r border-panel-border min-h-0 flex flex-col">
              <div className="px-2 pt-2 pb-1 flex items-center gap-2">
                <button
                  onClick={() => isAllSelected ? deselectAll() : selectAll()}
                  className="p-0.5 text-muted-foreground hover:text-terminal-fg"
                  aria-label={isAllSelected ? "Deselect all" : "Select all"}
                >
                  {isAllSelected ? (
                    <CheckSquare className="w-3.5 h-3.5 text-terminal-blue" />
                  ) : isSomeSelected ? (
                    <Minus className="w-3.5 h-3.5 text-terminal-blue" />
                  ) : (
                    <Square className="w-3.5 h-3.5" />
                  )}
                </button>
                <span className="text-[11px] uppercase tracking-wider text-muted-foreground">
                  Files ({files.length})
                </span>
                <div className="ml-auto flex items-center gap-1">
                  <div className="relative">
                    <button
                      onClick={() => setShowFilterDropdown(prev => !prev)}
                      className={`p-1 rounded text-muted-foreground hover:text-terminal-fg hover:bg-[#b9bcc01c] ${filterMode !== "all" ? "text-terminal-blue" : ""}`}
                      aria-label="Filter files"
                    >
                      <Filter className="w-3.5 h-3.5" />
                    </button>
                    {showFilterDropdown && (
                      <div className="absolute right-0 top-full mt-1 z-20 min-w-[160px] rounded border border-panel-border bg-panel-bg shadow-lg py-1">
                        {(["all", "pending", "reviewed", "approved", "rejected", "with-findings"] as FilterMode[]).map(mode => (
                          <button
                            key={mode}
                            onClick={() => { setFilterMode(mode); setShowFilterDropdown(false) }}
                            className={`w-full text-left px-3 py-1.5 text-xs hover:bg-[#b9bcc01c] flex items-center justify-between ${filterMode === mode ? "text-terminal-blue" : "text-terminal-fg"}`}
                          >
                            <span className="capitalize">{mode === "with-findings" ? "With Findings" : mode}</span>
                            <span className="text-[10px] text-muted-foreground">{statusCounts[mode]}</span>
                          </button>
                        ))}
                      </div>
                    )}
                  </div>
                  <button
                    onClick={() => setSortMode(prev => prev === "name" ? "status" : prev === "status" ? "findings" : "name")}
                    className="p-1 rounded text-[10px] text-muted-foreground hover:text-terminal-fg hover:bg-[#b9bcc01c]"
                    title={`Sort: ${sortMode}`}
                  >
                    {sortMode === "name" ? "A-Z" : sortMode === "status" ? "STS" : "FND"}
                  </button>
                </div>
              </div>

              {isReviewThinking && (
                <div className="mx-2 mb-1 h-1 rounded-full bg-terminal-blue/20 overflow-hidden">
                  <div className="h-full w-1/3 bg-terminal-blue rounded-full animate-pulse" style={{ animation: "pulse 1.5s ease-in-out infinite, slideRight 2s ease-in-out infinite" }} />
                </div>
              )}

              {isReviewComplete && !isReviewThinking && (
                <div className="mx-2 mb-1 px-2 py-1.5 rounded border border-terminal-green/20 bg-terminal-green/5 text-[11px] text-terminal-green flex items-center gap-1.5">
                  <Check className="w-3 h-3" />
                  <span>
                    {reviewSummary.totalFiles} files reviewed
                    {reviewSummary.totalFindings > 0 && <> | {reviewSummary.totalFindings} finding{reviewSummary.totalFindings > 1 ? "s" : ""}</>}
                    {reviewSummary.reviewedCount > 0 && <> | {reviewSummary.reviewedCount} auto-approved</>}
                  </span>
                </div>
              )}

              <div className="flex-1 min-h-0 overflow-y-auto px-2 pb-2">
                {displayFiles.map((file) => {
                  const decision = decisions[file.name] ?? "pending"
                  const isActive = selectedPath === file.name
                  const isChecked = selectedPaths.has(file.name)
                  const findings = findingsCountByFile[file.name] ?? 0
                  const DecisionIcon = decisionConfig[decision].icon

                  return (
                    <button
                      key={file.name}
                      onClick={() => setSelectedPath(file.name)}
                      className={`w-full text-left rounded px-2 py-1.5 mb-0.5 border transition-colors ${
                        isActive
                          ? "bg-sidebar-accent border-sidebar-accent"
                          : "border-transparent hover:bg-[#b9bcc01c]"
                      }`}
                    >
                      <div className="flex items-center gap-1.5">
                        <span
                          role="checkbox"
                          aria-checked={isChecked}
                          onClick={(e) => { e.stopPropagation(); toggleFileSelection(file.name) }}
                          className="flex-shrink-0 cursor-pointer text-muted-foreground hover:text-terminal-fg"
                        >
                          {isChecked ? (
                            <CheckSquare className="w-3.5 h-3.5 text-terminal-blue" />
                          ) : (
                            <Square className="w-3.5 h-3.5" />
                          )}
                        </span>
                        <span className={`text-[10px] font-bold w-3 flex-shrink-0 ${statusClass[file.status]}`}>
                          {statusLabel[file.status]}
                        </span>
                        <span className="text-xs text-terminal-fg truncate flex-1 min-w-0">{file.name}</span>
                        <DecisionIcon className={`w-3 h-3 flex-shrink-0 ${decisionConfig[decision].color}`} />
                        {findings > 0 && (
                          <span className="flex-shrink-0 text-[9px] bg-terminal-gold/20 text-terminal-gold px-1 py-0.5 rounded-full leading-none">
                            {findings}
                          </span>
                        )}
                      </div>
                    </button>
                  )
                })}
              </div>

              {selectedPaths.size > 0 && (
                <div className="border-t border-panel-border px-2 py-2 flex items-center gap-2 bg-panel-bg">
                  <span className="text-[11px] text-muted-foreground">{selectedPaths.size} selected</span>
                  <div className="ml-auto flex items-center gap-1">
                    <button
                      onClick={batchApprove}
                      className="flex items-center gap-1 px-2 py-1 rounded text-[11px] bg-terminal-green/15 text-terminal-green hover:bg-terminal-green/25"
                    >
                      <ShieldCheck className="w-3 h-3" />
                      Approve
                    </button>
                    <button
                      onClick={batchReject}
                      className="flex items-center gap-1 px-2 py-1 rounded text-[11px] bg-terminal-red/15 text-terminal-red hover:bg-terminal-red/25"
                    >
                      <ShieldAlert className="w-3 h-3" />
                      Reject
                    </button>
                    <button
                      onClick={batchReset}
                      className="px-2 py-1 rounded text-[11px] bg-background/50 text-muted-foreground hover:text-terminal-fg"
                    >
                      Reset
                    </button>
                  </div>
                </div>
              )}
            </div>

            <div className="min-h-0 flex flex-col">
              <div className="min-h-[240px] max-h-[46%] border-b border-panel-border flex flex-col">
                <div className="px-3 py-2 text-[11px] uppercase tracking-wider text-muted-foreground">
                  Review Feed
                </div>
                <div className="relative flex-1 min-h-0">
                  <div ref={feedRef} className="h-full overflow-y-auto border-t border-panel-border/60 bg-background/20 px-3 py-2">
                    {reviewFeedEntries.length === 0 && !isReviewThinking ? (
                      <div className="py-3 text-xs text-muted-foreground">
                        No review updates yet. Run <code>/review</code> to start.
                      </div>
                    ) : (
                      reviewFeedEntries.map((entry) => (
                        <div key={entry.id} className="mb-3">
                          <div className="text-[11px] text-muted-foreground mb-1">{entry.label}</div>
                          {entry.files.map((file, index) => (
                            <DiffViewer
                              key={`${entry.id}-${file.filename}-${index}`}
                              filename={file.filename}
                              lines={file.lines}
                              className="my-1"
                            />
                          ))}
                        </div>
                      ))
                    )}
                    {isReviewThinking && (
                      <div className="rounded border border-panel-border/70 bg-background/50 px-2 py-1.5 text-[11px] text-muted-foreground">
                        Alicia is reviewing changes...
                      </div>
                    )}
                  </div>
                  {hasPendingFeedItems > 0 && !feedAtBottom && (
                    <div className="pointer-events-none absolute bottom-3 right-3">
                      <button
                        onClick={scrollFeedToBottom}
                        className="pointer-events-auto rounded border border-terminal-blue/30 bg-terminal-blue/15 px-2 py-1 text-[11px] text-terminal-blue hover:bg-terminal-blue/25"
                      >
                        {hasPendingFeedItems} new update{hasPendingFeedItems > 1 ? "s" : ""}
                      </button>
                    </div>
                  )}
                </div>
              </div>

              {selectedFile ? (
                <div className="flex-1 min-h-0 flex flex-col">
                  <div className="p-3 border-b border-panel-border flex items-center gap-2">
                    <span className={`text-xs font-bold ${statusClass[selectedFile.status]}`}>
                      {statusLabel[selectedFile.status]}
                    </span>
                    <span className="text-sm text-terminal-fg truncate">{selectedFile.name}</span>
                    {(() => {
                      const dec = decisions[selectedFile.name] ?? "pending"
                      const cfg = decisionConfig[dec]
                      const Icon = cfg.icon
                      return (
                        <span className={`flex items-center gap-1 text-[11px] ${cfg.color}`}>
                          <Icon className="w-3.5 h-3.5" />
                          {cfg.label}
                        </span>
                      )
                    })()}
                    <span className="ml-auto text-[10px] text-terminal-green bg-terminal-green/10 px-1.5 py-0.5 rounded">
                      +{selectedDiffStats.additions}
                    </span>
                    <span className="text-[10px] text-terminal-red bg-terminal-red/10 px-1.5 py-0.5 rounded">
                      -{selectedDiffStats.removals}
                    </span>
                  </div>

                  <div className="p-3 border-b border-panel-border flex flex-wrap items-center gap-2">
                    <button
                      onClick={() =>
                        setDecisions((previous) => ({ ...previous, [selectedFile.name]: "approved" }))
                      }
                      disabled={selectedFileHasConflict}
                      className={`flex items-center gap-1 px-2 py-1 rounded text-xs transition-colors ${
                        decisions[selectedFile.name] === "approved"
                          ? "bg-terminal-green/20 text-terminal-green"
                          : "bg-background/50 text-muted-foreground hover:text-terminal-green"
                      } disabled:opacity-50 disabled:cursor-not-allowed`}
                    >
                      <ShieldCheck className="w-3.5 h-3.5" />
                      Approve
                    </button>
                    <button
                      onClick={() =>
                        setDecisions((previous) => ({ ...previous, [selectedFile.name]: "rejected" }))
                      }
                      className={`flex items-center gap-1 px-2 py-1 rounded text-xs transition-colors ${
                        decisions[selectedFile.name] === "rejected"
                          ? "bg-terminal-red/20 text-terminal-red"
                          : "bg-background/50 text-muted-foreground hover:text-terminal-red"
                      }`}
                    >
                      <ShieldAlert className="w-3.5 h-3.5" />
                      Reject
                    </button>
                    <button
                      onClick={() =>
                        setDecisions((previous) => ({ ...previous, [selectedFile.name]: "pending" }))
                      }
                      className="px-2 py-1 rounded text-xs bg-background/50 text-muted-foreground hover:text-terminal-fg"
                    >
                      Reset
                    </button>
                    {selectedFileHasConflict && (
                      <span className="text-[11px] text-terminal-red">
                        Resolve git conflicts before approval/commit.
                      </span>
                    )}
                  </div>

                  <div className="px-3 py-2 border-b border-panel-border">
                    <button
                      onClick={() =>
                        setExpandedComments((previous) => ({
                          ...previous,
                          [selectedFile.name]: !previous[selectedFile.name],
                        }))
                      }
                      className="text-[11px] uppercase tracking-wider text-muted-foreground hover:text-terminal-fg"
                    >
                      {expandedComments[selectedFile.name] ? "Hide comments" : "Show comments"}
                    </button>
                    {expandedComments[selectedFile.name] ? (
                      <textarea
                        value={comments[selectedFile.name] ?? ""}
                        onChange={(event) => {
                          const nextValue = event.target.value
                          const autoValue = reviewCommentsByFile[selectedFile.name] ?? ""
                          setComments((previous) => ({
                            ...previous,
                            [selectedFile.name]: nextValue,
                          }))
                          setManualCommentEdits((previous) => ({
                            ...previous,
                            [selectedFile.name]: nextValue.trim() !== autoValue.trim(),
                          }))
                        }}
                        placeholder="Add review notes for this file..."
                        className="mt-2 w-full min-h-[72px] rounded border border-panel-border bg-background/60 px-2 py-1.5 text-xs text-terminal-fg outline-none focus:border-terminal-blue/40"
                      />
                    ) : (
                      <div className="mt-1 text-[11px] text-muted-foreground">
                        {(comments[selectedFile.name] ?? "").trim().length > 0
                          ? "Comment saved"
                          : "No comments"}
                      </div>
                    )}
                  </div>

                  <div className="flex-1 min-h-0 overflow-auto px-3 pb-3">
                    {selectedFile.diff && selectedFile.diff.lines.length > 0 ? (
                      <DiffViewer
                        filename={selectedFile.name}
                        lines={selectedFile.diff.lines}
                        className="my-3"
                      />
                    ) : (
                      <div className="py-3 text-xs text-muted-foreground">
                        No parsed diff available for this file.
                      </div>
                    )}
                  </div>
                </div>
              ) : null}
            </div>
          </div>
        )}

        <div className={`border-t p-3 flex flex-col gap-2 ${
          isReviewComplete && reviewSummary.pendingCount === 0 && reviewSummary.rejectedCount === 0
            ? "border-terminal-green/30 bg-terminal-green/5"
            : "border-panel-border"
        }`}>
          {isReviewComplete && reviewSummary.pendingCount === 0 && reviewSummary.rejectedCount === 0 && (
            <div className="text-[11px] text-terminal-green flex items-center gap-1.5">
              <CheckCheck className="w-3.5 h-3.5" />
              All files reviewed. Ready to commit.
            </div>
          )}

          {pendingApprovals.length > 0 && (
            <div className="text-xs text-muted-foreground bg-background/40 border border-panel-border rounded px-2 py-1.5">
              Pending approvals: {pendingApprovals.length}
            </div>
          )}

          <div className="flex items-center gap-2">
            <input
              value={commitMessage}
              onChange={(event) => setCommitMessage(event.target.value)}
              placeholder="Commit message"
              className="flex-1 rounded border border-panel-border bg-background/60 px-2 py-1.5 text-xs text-terminal-fg outline-none focus:border-terminal-blue/40"
            />
            <button
              onClick={handleCommitApproved}
              disabled={!canCommit}
              className="flex items-center gap-1 rounded px-3 py-1.5 text-xs bg-terminal-green/15 text-terminal-green disabled:opacity-50 disabled:cursor-not-allowed hover:bg-terminal-green/25"
            >
              <Check className="w-3.5 h-3.5" />
              Commit ({approvedPaths.length})
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
