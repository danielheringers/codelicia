param(
    [string]$VbsRoot = "C:/Users/danie/OneDrive/Documentos/Projetos/vibing-steampunk",
    [string]$NeuroRoot = "C:/Users/danie/OneDrive/Documentos/Projetos/Neuromancer/codex/codex-rs"
)

$ErrorActionPreference = "Stop"

function Extract-ToolsFromVbs {
    param([string]$ConfigFile)

    $raw = Get-Content -Path $ConfigFile -Raw
    $startToken = "func GetAllToolNames() []string {"
    $endToken = "func GetFocusedToolNames() []string {"
    $startIndex = $raw.IndexOf($startToken)
    if ($startIndex -lt 0) {
        throw "Could not locate GetAllToolNames() in $ConfigFile"
    }
    $endIndex = $raw.IndexOf($endToken, $startIndex)
    if ($endIndex -lt 0) {
        throw "Could not locate end boundary for GetAllToolNames() in $ConfigFile"
    }

    $listRaw = $raw.Substring($startIndex, $endIndex - $startIndex)
    $matches = [regex]::Matches($listRaw, '"(?<name>[A-Za-z0-9]+)"')
    $tools = New-Object System.Collections.Generic.List[string]
    foreach ($m in $matches) {
        $name = $m.Groups["name"].Value
        if (-not [string]::IsNullOrWhiteSpace($name)) {
            $tools.Add($name)
        }
    }
    return $tools | Select-Object -Unique
}

function Extract-AdtEndpoints {
    param(
        [string]$AdtDir,
        [string]$VbsRoot
    )

    $files = Get-ChildItem -Path $AdtDir -Filter *.go -File -Recurse |
        Where-Object { $_.Name -notlike "*_test.go" }

    $byPath = @{}
    foreach ($file in $files) {
        $raw = Get-Content -Path $file.FullName -Raw
        $pathMatches = [regex]::Matches($raw, '"(?<path>/sap/bc/adt[^"\s]*)"')
        foreach ($match in $pathMatches) {
            $path = $match.Groups["path"].Value
            if (-not $byPath.ContainsKey($path)) {
                $byPath[$path] = [ordered]@{
                    path = $path
                    files = New-Object System.Collections.Generic.HashSet[string]
                    methods = New-Object System.Collections.Generic.HashSet[string]
                    occurrences = 0
                }
            }
            $relativeFile = $file.FullName.Replace('\', '/')
            $vbsRootNormalized = $VbsRoot.Replace('\', '/').TrimEnd('/')
            if ($relativeFile.StartsWith("$vbsRootNormalized/")) {
                $relativeFile = $relativeFile.Substring($vbsRootNormalized.Length + 1)
            }
            $null = $byPath[$path].files.Add($relativeFile)
            $byPath[$path].occurrences++
        }

        $requestMatches = [regex]::Matches(
            $raw,
            'Request\(\s*ctx\s*,\s*"(?<path>/sap/bc/adt[^"]+)"\s*,\s*&RequestOptions\s*\{(?<opts>[\s\S]*?)\}\s*\)',
            [System.Text.RegularExpressions.RegexOptions]::Singleline
        )
        foreach ($request in $requestMatches) {
            $path = $request.Groups["path"].Value
            if (-not $byPath.ContainsKey($path)) {
                continue
            }
            $opts = $request.Groups["opts"].Value
            $methodMatch = [regex]::Match($opts, 'Method:\s*http\.Method(?<method>[A-Za-z]+)')
            if ($methodMatch.Success) {
                $method = $methodMatch.Groups["method"].Value.ToUpperInvariant()
                $null = $byPath[$path].methods.Add($method)
            }
        }
    }

    $result = @()
    foreach ($entry in ($byPath.Values | Sort-Object path)) {
        $result += [ordered]@{
            path = $entry.path
            methods = @($entry.methods | Sort-Object)
            occurrences = $entry.occurrences
            files = @($entry.files | Sort-Object)
        }
    }
    return $result
}

function Extract-NeuroMcpToolNames {
    param([string]$NeuroMcpLib)
    $raw = Get-Content -Path $NeuroMcpLib -Raw

    $names = New-Object System.Collections.Generic.List[string]
    $constMatches = [regex]::Matches(
        $raw,
        'const (VBS_TOOL_NAMES|NEURO_INTERNAL_TOOL_NAMES):\s*&\[\&str\]\s*=\s*&\[(?<body>[\s\S]*?)\];',
        [System.Text.RegularExpressions.RegexOptions]::Singleline
    )
    foreach ($constMatch in $constMatches) {
        $body = $constMatch.Groups["body"].Value
        $toolMatches = [regex]::Matches($body, '"(?<name>[A-Za-z0-9_]+)"')
        foreach ($tool in $toolMatches) {
            $names.Add($tool.Groups["name"].Value)
        }
    }
    return $names | Select-Object -Unique
}

function Extract-NeuroMcpImplementedToolNames {
    param([string]$NeuroMcpLib)
    $raw = Get-Content -Path $NeuroMcpLib -Raw
    $match = [regex]::Match(
        $raw,
        'const IMPLEMENTED_TOOL_NAMES:\s*&\[\&str\]\s*=\s*&\[(?<body>[\s\S]*?)\];',
        [System.Text.RegularExpressions.RegexOptions]::Singleline
    )
    if (-not $match.Success) {
        return @()
    }

    $body = $match.Groups["body"].Value
    $toolMatches = [regex]::Matches($body, '"(?<name>[A-Za-z0-9_]+)"')
    $names = New-Object System.Collections.Generic.List[string]
    foreach ($tool in $toolMatches) {
        $names.Add($tool.Groups["name"].Value)
    }
    return $names | Select-Object -Unique
}

$toolsConfigPath = Join-Path $VbsRoot "cmd/vsp/config_cmd.go"
$adtDir = Join-Path $VbsRoot "pkg/adt"
$neuroMcpLib = Join-Path $NeuroRoot "neuro-mcp/src/lib.rs"

$outDir = Join-Path $NeuroRoot "docs/parity"
$contractDir = Join-Path $outDir "vbs-contract"
New-Item -ItemType Directory -Path $contractDir -Force | Out-Null

$tools = Extract-ToolsFromVbs -ConfigFile $toolsConfigPath
$endpoints = Extract-AdtEndpoints -AdtDir $adtDir -VbsRoot $VbsRoot
$neuroTools = Extract-NeuroMcpToolNames -NeuroMcpLib $neuroMcpLib
$implementedTools = Extract-NeuroMcpImplementedToolNames -NeuroMcpLib $neuroMcpLib

$generatedAt = [DateTime]::UtcNow.ToString("o")

$toolsPayload = [ordered]@{
    generated_at_utc = $generatedAt
    vbs_source = "cmd/vsp/config_cmd.go"
    count = $tools.Count
    tools = $tools
}

$endpointsPayload = [ordered]@{
    generated_at_utc = $generatedAt
    vbs_source_dir = "pkg/adt"
    count = $endpoints.Count
    endpoints = $endpoints
}

$toolsPayload | ConvertTo-Json -Depth 6 | Set-Content -Path (Join-Path $contractDir "tools.json")
$endpointsPayload | ConvertTo-Json -Depth 8 | Set-Content -Path (Join-Path $contractDir "endpoints.json")

$neuroSet = New-Object System.Collections.Generic.HashSet[string] ([StringComparer]::OrdinalIgnoreCase)
foreach ($tool in $neuroTools) {
    $null = $neuroSet.Add($tool)
}
$implementedSet = New-Object System.Collections.Generic.HashSet[string] ([StringComparer]::OrdinalIgnoreCase)
foreach ($tool in $implementedTools) {
    $null = $implementedSet.Add($tool)
}

$catalogMatched = New-Object System.Collections.Generic.List[string]
$catalogMissing = New-Object System.Collections.Generic.List[string]
$functionallyImplemented = New-Object System.Collections.Generic.List[string]
$functionallyMissing = New-Object System.Collections.Generic.List[string]
foreach ($tool in $tools) {
    if ($neuroSet.Contains($tool)) {
        $catalogMatched.Add($tool)
    } else {
        $catalogMissing.Add($tool)
    }

    if ($implementedSet.Contains($tool)) {
        $functionallyImplemented.Add($tool)
    } else {
        $functionallyMissing.Add($tool)
    }
}

$gapLines = @()
$gapLines += "# VBS -> Neuro Gap List"
$gapLines += ""
$gapLines += "- generated_at_utc: $generatedAt"
$gapLines += "- vbs_tools_total: $($tools.Count)"
$gapLines += "- neuro_mcp_catalog_matched: $($catalogMatched.Count)"
$gapLines += "- neuro_mcp_catalog_missing: $($catalogMissing.Count)"
$gapLines += "- neuro_mcp_functionally_implemented: $($functionallyImplemented.Count)"
$gapLines += "- neuro_mcp_functionally_missing: $($functionallyMissing.Count)"
$gapLines += ""
$gapLines += "## Catalog Matched"
$gapLines += ""
if ($catalogMatched.Count -eq 0) {
    $gapLines += "_none_"
} else {
    foreach ($tool in ($catalogMatched | Sort-Object)) {
        $gapLines += "- $tool"
    }
}
$gapLines += ""
$gapLines += "## Catalog Missing"
$gapLines += ""
if ($catalogMissing.Count -eq 0) {
    $gapLines += "_none_"
} else {
    foreach ($tool in ($catalogMissing | Sort-Object)) {
        $gapLines += "- $tool"
    }
}
$gapLines += ""
$gapLines += "## Functionally Implemented"
$gapLines += ""
if ($functionallyImplemented.Count -eq 0) {
    $gapLines += "_none_"
} else {
    foreach ($tool in ($functionallyImplemented | Sort-Object)) {
        $gapLines += "- $tool"
    }
}
$gapLines += ""
$gapLines += "## Functionally Missing"
$gapLines += ""
if ($functionallyMissing.Count -eq 0) {
    $gapLines += "_none_"
} else {
    foreach ($tool in ($functionallyMissing | Sort-Object)) {
        $gapLines += "- $tool"
    }
}

$gapLines | Set-Content -Path (Join-Path $outDir "gap-list.md")

Write-Host "Generated:"
Write-Host " - $contractDir/tools.json"
Write-Host " - $contractDir/endpoints.json"
Write-Host " - $outDir/gap-list.md"
