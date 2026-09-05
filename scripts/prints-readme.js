// Tira os prints da interface para o README, contra o servidor de
// desenvolvimento (http://localhost:1420), que roda com dados de demonstração.
// Uso: node scripts/prints-readme.js
const { chromium } = require('./node_modules/playwright');
const fs = require('fs');
const path = require('path');

const SAIDA = path.join(__dirname, '..', 'docs', 'imagens');
const URL = 'http://localhost:1420';

(async () => {
  fs.mkdirSync(SAIDA, { recursive: true });
  const navegador = await chromium.launch();
  const pagina = await navegador.newPage({
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 2,
    colorScheme: 'dark',
  });
  await pagina.goto(URL, { waitUntil: 'networkidle' });
  await pagina.waitForSelector('svg[aria-label]');
  await pagina.waitForTimeout(500);

  // 1. Tela principal, sem nada selecionado.
  await pagina.screenshot({ path: path.join(SAIDA, 'tela-principal.png') });

  // 2. Só o desenho do aparelho.
  await pagina.locator('svg[aria-label]').screenshot({ path: path.join(SAIDA, 'aparelho.png') });

  // 3. Painel de um pad programado (o 13, Chrome na demonstração).
  await pagina.click('[data-alvo="pad:13"]');
  await pagina.waitForTimeout(300);
  await pagina.screenshot({ path: path.join(SAIDA, 'painel-do-pad.png') });

  // 4. Configurações.
  await pagina.getByRole('button', { name: 'Configurações' }).click();
  await pagina.waitForTimeout(400);
  await pagina.screenshot({ path: path.join(SAIDA, 'configuracoes.png') });

  await navegador.close();
  for (const f of fs.readdirSync(SAIDA)) {
    const { size } = fs.statSync(path.join(SAIDA, f));
    console.log(f, Math.round(size / 1024) + ' KB');
  }
})().catch((e) => {
  console.error(e.message);
  process.exit(1);
});
