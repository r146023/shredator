function Invoke-Shredator {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$Target,

        [string]$Binary = "shredator.exe",

        [int]$Passes = 3,

        [ValidateSet("random", "zeros", "ones", "alt", "alternating", "dod", "gutmann")]
        [string]$Pattern = "random",

        [switch]$AllowWarnings
    )

    $output = & $Binary $Target --force --passes $Passes --pattern $Pattern --json 2>&1
    $code = $LASTEXITCODE

    try {
        $payload = $output | ConvertFrom-Json
    }
    catch {
        throw "Shredator did not emit valid JSON. ExitCode=$code Output=$output"
    }

    if ($payload.exit_code -ne $code) {
        throw "Exit code mismatch. Process=$code JSON=$($payload.exit_code)"
    }

    if (-not $payload.success) {
        throw "Shredator failed: $($payload | ConvertTo-Json -Depth 20)"
    }

    if (-not $AllowWarnings -and $payload.summary.warnings -gt 0) {
        throw "Shredator completed with warnings: $($payload | ConvertTo-Json -Depth 20)"
    }

    return $payload
}
