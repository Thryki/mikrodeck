// Gera imagens de comparação com grade em unidades do viewBox (960x540) sobre render e foto.
// Saída em design/comparacao/: render-grade.png, foto-grade.png, foto-grade-esquerda.png,
// foto-grade-direita.png, lado-a-lado.png
const sharp = require('sharp');
const path = require('path');
const fs = require('fs');
const root = path.resolve(__dirname, '..');
const render = path.join(root, 'design/mikro-mk3.png');
const foto = path.join(root, 'design/referencia/mikro-mk3-foto-hd.png');
const dir = path.join(root, 'design/comparacao');
fs.mkdirSync(dir, { recursive: true });
const VBW = 960, VBH = 540;

function gridSvg(w, h, fontPx) {
  let s = `<svg width="${w}" height="${h}" xmlns="http://www.w3.org/2000/svg" font-family="Arial" font-size="${fontPx}">`;
  for (let p = 0; p <= 100; p += 5) {
    const major = p % 10 === 0;
    const x = (w * p) / 100, y = (h * p) / 100;
    const col = major ? '#ffe600' : '#ffe60066';
    s += `<line x1="${x}" y1="0" x2="${x}" y2="${h}" stroke="${col}" stroke-width="${major ? 2 : 1}"/>`;
    s += `<line x1="0" y1="${y}" x2="${w}" y2="${y}" stroke="${col}" stroke-width="${major ? 2 : 1}"/>`;
    if (major && p < 100) {
      const vx = Math.round((VBW * p) / 100), vy = Math.round((VBH * p) / 100);
      const bw = fontPx * 3.4, bh = fontPx * 1.3;
      for (const yy of [2, h - bh - 2]) s += `<rect x="${x + 2}" y="${yy}" width="${bw}" height="${bh}" fill="#000000aa"/><text x="${x + 5}" y="${yy + fontPx}" fill="#ffe600">x${vx}</text>`;
      for (const xx of [2, w - bw - 2]) s += `<rect x="${xx}" y="${y + 2}" width="${bw}" height="${bh}" fill="#000000aa"/><text x="${xx + 3}" y="${y + 2 + fontPx}" fill="#ffe600">y${vy}</text>`;
    }
  }
  return Buffer.from(s + '</svg>');
}
async function withGrid(input, w, h, fontPx) {
  return sharp(input).resize({ width: w, height: h, fit: 'fill' }).composite([{ input: gridSvg(w, h, fontPx), top: 0, left: 0 }]).png().toBuffer();
}
(async () => {
  const W = 1920, H = 1080;
  const r = await withGrid(render, W, H, 16);
  const f = await withGrid(foto, W, H, 16);
  fs.writeFileSync(path.join(dir, 'render-grade.png'), r);
  fs.writeFileSync(path.join(dir, 'foto-grade.png'), f);
  // Foto em alta com grade, dividida em metades com sobreposição, para ler rótulos
  const FW = 3456, FH = 1944;
  const fhd = await withGrid(foto, FW, FH, 26);
  const half = FW / 2, ov = 200;
  await sharp(fhd).extract({ left: 0, top: 0, width: half + ov, height: FH }).png().toFile(path.join(dir, 'foto-grade-esquerda.png'));
  await sharp(fhd).extract({ left: half - ov, top: 0, width: half + ov, height: FH }).png().toFile(path.join(dir, 'foto-grade-direita.png'));
  const gap = 30;
  const label = (t) => Buffer.from(`<svg width="${W}" height="${gap}"><rect width="${W}" height="${gap}" fill="#fff"/><text x="10" y="22" font-family="Arial" font-size="20" fill="#000">${t}</text></svg>`);
  await sharp({ create: { width: W, height: gap + H + gap + H, channels: 3, background: '#ffffff' } })
    .composite([
      { input: label('RENDER (design/mikro-mk3.png)'), top: 0, left: 0 }, { input: r, top: gap, left: 0 },
      { input: label('FOTO (design/referencia/mikro-mk3-foto-hd.png)'), top: gap + H, left: 0 }, { input: f, top: gap + H + gap, left: 0 },
    ]).png().toFile(path.join(dir, 'lado-a-lado.png'));
  console.log('Gerado: design/comparacao/{render-grade,foto-grade,foto-grade-esquerda,foto-grade-direita,lado-a-lado}.png');
})().catch((e) => { console.error(e); process.exit(1); });
