// Renderiza até duas linhas de texto num bitmap de 128x32 para a tela do Mikro MK3.
// Saída: arquivo binário de 512 bytes, no formato que o report 0xe0 espera.
// Uso: node scripts/tela-texto.js "linha 1" "linha 2" saida.bin
const sharp = require('./node_modules/sharp');
const fs = require('fs');

const linha1 = process.argv[2] || '';
const linha2 = process.argv[3] || '';
const saida = process.argv[4] || 'tela.bin';
const L = 128, A = 32;

const escapar = (t) => t.replace(/[&<>]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' })[c]);
const svg = `<svg width="${L}" height="${A}" xmlns="http://www.w3.org/2000/svg">
  <rect width="${L}" height="${A}" fill="black"/>
  <text x="2" y="13" font-family="DejaVu Sans, Verdana, Arial" font-size="13" font-weight="bold" fill="white">${escapar(linha1)}</text>
  <text x="2" y="29" font-family="DejaVu Sans, Verdana, Arial" font-size="13" fill="white">${escapar(linha2)}</text>
</svg>`;

(async () => {
  const { data } = await sharp(Buffer.from(svg)).greyscale().raw().toBuffer({ resolveWithObject: true });
  const aceso = (x, y) => data[y * L + x] > 127;

  // Empacota como o pymikro: por coluna, 4 bytes de 8 linhas.
  // Bit 1 significa pixel APAGADO, por isso a inversão.
  const buf = Buffer.alloc(512);
  for (let x = 0; x < L; x++) {
    for (let bloco = 0; bloco < 4; bloco++) {
      let b = 0;
      for (let bit = 0; bit < 8; bit++) {
        b <<= 1;
        if (!aceso(x, bloco * 8 + (7 - bit))) b += 1;
      }
      buf[128 * bloco + x] = b;
    }
  }
  fs.writeFileSync(saida, buf);
  console.log(`Gerado: ${saida} (512 bytes) para "${linha1}" / "${linha2}"`);
})();
