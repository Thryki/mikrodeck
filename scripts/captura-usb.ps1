# Captura o tráfego USB do Maschine Mikro MK3 enquanto mandamos pacotes conhecidos.
# Precisa rodar como administrador (o USBPcap exige).
#
# O que faz:
#  1. Descobre em qual root hub o aparelho está.
#  2. Começa a capturar.
#  3. Manda um pacote de tela (report 0xe0, que funciona) e um de LED (report 0x80, que não funciona).
#  4. Para a captura e salva o .pcap.
#
# Assim dá para comparar, no fio, quantos bytes o Windows realmente envia em cada caso.

$ErrorActionPreference = 'Stop'
$raiz = Split-Path -Parent $PSScriptRoot
$usbpcap = 'C:\Program Files\USBPcap\USBPcapCMD.exe'
$envia = Join-Path $raiz 'mikrodeck\spike-hid\target\debug\envia.exe'
$saida = '$env:USERPROFILE\AppData\Local\Temp\mikrodeck-captura'
New-Item -ItemType Directory -Force -Path $saida | Out-Null

# Grava tudo num log, para o Claude conseguir ler o que aconteceu mesmo se a janela fechar.
Start-Transcript -Path (Join-Path $saida 'log.txt') -Force | Out-Null
trap {
    Write-Host "ERRO: $_" -ForegroundColor Red
    Write-Host $_.ScriptStackTrace
    try { Stop-Transcript | Out-Null } catch {}
    Start-Sleep -Seconds 20
    exit 1
}

if (-not (Test-Path $usbpcap)) { throw "USBPcap não encontrado em $usbpcap" }
if (-not (Test-Path $envia)) { throw "envia.exe não encontrado em $envia. Compile antes." }

Write-Host "=== Root hubs USB disponíveis ===" -ForegroundColor Cyan
$listagem = & $usbpcap --extcap-interfaces 2>&1 | Out-String
Write-Host $listagem

# Extrai os nomes \\.\USBPcapN da listagem
$hubs = [regex]::Matches($listagem, '\\\\\.\\USBPcap\d+') | ForEach-Object { $_.Value } | Select-Object -Unique
if (-not $hubs) { throw "Nenhum root hub USB encontrado. Está rodando como administrador?" }
Write-Host "Hubs: $($hubs -join ', ')" -ForegroundColor Yellow

# Captura em todos os hubs ao mesmo tempo, para não errar qual é o do aparelho.
$procs = @()
foreach ($h in $hubs) {
    $n = ($h -replace '[\\\.]', '')
    $arq = Join-Path $saida "$n.pcap"
    Remove-Item $arq -ErrorAction SilentlyContinue
    $p = Start-Process -FilePath $usbpcap -ArgumentList @('-d', $h, '-o', $arq, '-A') -PassThru -WindowStyle Hidden
    $procs += [pscustomobject]@{ Proc = $p; Arquivo = $arq; Hub = $h }
}
Write-Host "Capturando em $($procs.Count) hub(s)..." -ForegroundColor Green
Start-Sleep -Seconds 2

Write-Host "`n--- Mandando pacote de TELA (report 0xe0, sabidamente funciona) ---" -ForegroundColor Cyan
& $envia --tela on
Start-Sleep -Seconds 1
& $envia --tela off
Start-Sleep -Seconds 1

Write-Host "`n--- Mandando pacote de LED (report 0x80, não funciona) ---" -ForegroundColor Cyan
& $envia --preenche 0x80 0x07 --tam 81
Start-Sleep -Seconds 1
& $envia --preenche 0x80 0x1F --tam 81
Start-Sleep -Seconds 2

Write-Host "`nParando captura..." -ForegroundColor Green
foreach ($x in $procs) {
    try { Stop-Process -Id $x.Proc.Id -Force -ErrorAction Stop } catch {}
}
Start-Sleep -Seconds 1

Write-Host "`n=== Arquivos gerados ===" -ForegroundColor Cyan
foreach ($x in $procs) {
    if (Test-Path $x.Arquivo) {
        $tam = (Get-Item $x.Arquivo).Length
        Write-Host ("{0}  {1} bytes" -f $x.Arquivo, $tam)
    }
}
Write-Host "`nPasta: $saida" -ForegroundColor Yellow
Write-Host "Pronto. Pode fechar esta janela." -ForegroundColor Green
try { Stop-Transcript | Out-Null } catch {}
Start-Sleep -Seconds 5
