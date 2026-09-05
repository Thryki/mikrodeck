// Recorta regiões de uma foto do aparelho, já corrigindo a rotação da webcam.
// Uso: node scripts/recorta.js <foto> <saida> <left> <top> <largura> <altura> [escala]
const sharp = require('./node_modules/sharp');
const [foto, saida, l, t, w, h, escala] = process.argv.slice(2);
(async () => {
  const reto = await sharp(foto).rotate(180).png().toBuffer();
  let img = sharp(reto).extract({ left: +l, top: +t, width: +w, height: +h });
  if (escala) img = img.resize(+escala, null, { kernel: 'nearest' });
  await img.toFile(saida);
  console.log(saida);
})();
