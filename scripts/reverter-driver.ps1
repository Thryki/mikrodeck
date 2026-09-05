# Reverte a interface MI_00 do Maschine Mikro MK3 de WinUSB para o driver HID original.
# Precisa de administrador.
#
# O Zadig instala um pacote de driver da libwdi. Removendo o pacote, o Windows reinstala
# sozinho o driver HID padrão, e o software da Native Instruments volta a enxergar o aparelho.

$saida = '$env:USERPROFILE\AppData\Local\Temp\mikrodeck-captura'
New-Item -ItemType Directory -Force -Path $saida | Out-Null
Start-Transcript -Path (Join-Path $saida 'reverter.txt') -Force | Out-Null

Write-Host "=== Estado antes ==="
Get-PnpDevice | Where-Object { $_.InstanceId -match 'VID_17CC' } |
    Select-Object Status, Class, FriendlyName | Format-Table -AutoSize

# Localiza o pacote instalado pelo Zadig (provedor libwdi, nome do aparelho no arquivo)
$alvo = $null
$saidaEnum = pnputil /enum-drivers | Out-String
foreach ($bloco in ($saidaEnum -split "(?m)^\s*$")) {
    if ($bloco -match 'maschine_mikro_mk3' -and $bloco -match 'libwdi') {
        if ($bloco -match '(oem\d+\.inf)') { $alvo = $matches[1] }
    }
}
if (-not $alvo) { $alvo = 'oem46.inf' }
Write-Host "`nPacote a remover: $alvo"

pnputil /delete-driver $alvo /uninstall /force
Write-Host "`nProcurando hardware novo..."
pnputil /scan-devices
Start-Sleep -Seconds 4

Write-Host "`n=== Estado depois ==="
Get-PnpDevice | Where-Object { $_.InstanceId -match 'VID_17CC' } |
    Select-Object Status, Class, FriendlyName | Format-Table -AutoSize

Write-Host "Se ainda aparecer USBDevice, desconecte e reconecte o cabo USB." -ForegroundColor Yellow
Stop-Transcript | Out-Null
Start-Sleep -Seconds 4
