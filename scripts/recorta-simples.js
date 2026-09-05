// Recorta uma região de um PNG. Uso: node scripts/recorta-simples.js <entrada> <saida> <l> <t> <w> <h> [escala]
const sharp = require('./node_modules/sharp');
const [e, s, l, t, w, h, escala] = process.argv.slice(2);
(async () => {
  let img = sharp(e).extract({ left: +l, top: +t, width: +w, height: +h });
  if (escala) img = img.resize(+escala);
  await img.toFile(s);
  console.log(s);
})();
