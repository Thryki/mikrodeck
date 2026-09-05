// Renderiza um SVG em PNG usando @resvg/resvg-js (fallback quando não há rsvg-convert/resvg).
// Uso: node scripts/render-svg.js [entrada.svg] [saida.png] [largura]
const { Resvg } = require('@resvg/resvg-js');
const fs = require('fs');
const path = require('path');
const root = path.resolve(__dirname, '..');
const inFile = process.argv[2] || path.join(root, 'design/mikro-mk3.svg');
const outFile = process.argv[3] || path.join(root, 'design/mikro-mk3.png');
const width = parseInt(process.argv[4] || '1920', 10);
const svg = fs.readFileSync(inFile, 'utf8');
const r = new Resvg(svg, {
  fitTo: { mode: 'width', value: width },
  font: {
    loadSystemFonts: true,
    defaultFontFamily: 'Segoe UI',
    sansSerifFamily: 'Segoe UI',
    monospaceFamily: 'Consolas',
  },
});
const img = r.render();
fs.writeFileSync(outFile, img.asPng());
console.log(`Gerado: ${path.relative(root, outFile)} (${img.width}x${img.height})`);
