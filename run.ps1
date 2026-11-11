# Usage:  .\run.ps1            (prompts for your API key if SUBSTREAMS_API_KEY is not set)
# Streams blocks 385870151..385870156 (stop block is exclusive -> 385870157)
# and prints one JSON object per transfer (also saved to transfers.jsonl).
param(
    [string]$Endpoint = "mainnet.sol.streamingfast.io:443",
    [long]$Start = 385870151,
    [long]$StopExclusive = 385870157
)
# Prefer a substreams.exe sitting next to this script (portable bundle).
$env:Path = "$PSScriptRoot;$env:USERPROFILE\.cargo\bin;$env:Path"
if (-not (Get-Command substreams -ErrorAction SilentlyContinue)) { throw "substreams.exe not found. Put it next to run.ps1." }

if (-not $env:SUBSTREAMS_API_KEY -and -not $env:SUBSTREAMS_API_TOKEN) {
    $secure = Read-Host "Paste your Substreams API key (input hidden)" -AsSecureString
    $env:SUBSTREAMS_API_KEY = [Runtime.InteropServices.Marshal]::PtrToStringAuto(
        [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure))
}

substreams run "$PSScriptRoot\solana-transfers-v0.1.0.spkg" map_transfers `
    -e $Endpoint -s $Start -t $StopExclusive -o jsonl |
  ForEach-Object {
    $line = $_ | ConvertFrom-Json
    foreach ($t in $line.'@data'.transfers) {
        '{"from": "' + $t.from + '", "to": "' + $t.to + '", "amount": ' + [decimal]$t.amount + '}'
    }
  } | Tee-Object -FilePath "$PSScriptRoot\transfers.jsonl"

Write-Host "`nDone. Transfers saved to $PSScriptRoot\transfers.jsonl"
