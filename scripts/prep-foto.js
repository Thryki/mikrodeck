// Prepara a foto HD de referência do Mikro MK3.
// Entrada: design/referencia/mikro-mk3-foto.DNG (JPEG do iPhone 2268x4032, aparelho fotografado na vertical)
// Passos: gira 90°, recorta com margem, corrige perspectiva por homografia (4 cantos do aparelho -> retângulo)
// Saída:  design/referencia/mikro-mk3-foto-hd.png (3456x1944 = viewBox 960x540 vezes 3.6; bordas = bordas do aparelho)
const sharp = require('sharp');
const path = require('path');
const root = path.resolve(__dirname, '..');
const src = path.join(root, 'design/referencia/mikro-mk3-foto.DNG');
const out = path.join(root, 'design/referencia/mikro-mk3-foto-hd.png');
const WIDE = { left: 250, top: 60, width: 3700, height: 2150 };
// Cantos do aparelho medidos no recorte WIDE (ver sessão de 2026-09-01): TL, TR, BR, BL
const CORNERS = [[137, 106], [3598, 206], [3586, 2066], [137, 2094]];
const DW = 3456, DH = 1944;

function solve(A, b) { // eliminação de Gauss, A 8x8
  const n = b.length; const M = A.map((r, i) => [...r, b[i]]);
  for (let c = 0; c < n; c++) {
    let p = c; for (let r = c + 1; r < n; r++) if (Math.abs(M[r][c]) > Math.abs(M[p][c])) p = r;
    [M[c], M[p]] = [M[p], M[c]];
    for (let r = 0; r < n; r++) { if (r === c) continue; const f = M[r][c] / M[c][c]; for (let k = c; k <= n; k++) M[r][k] -= f * M[c][k]; }
  }
  return M.map((r, i) => r[n] / r[i]);
}
// Homografia destino(u,v) -> origem(x,y)
function homography(dst, srcPts) {
  const A = [], b = [];
  for (let i = 0; i < 4; i++) {
    const [u, v] = dst[i], [x, y] = srcPts[i];
    A.push([u, v, 1, 0, 0, 0, -x * u, -x * v]); b.push(x);
    A.push([0, 0, 0, u, v, 1, -y * u, -y * v]); b.push(y);
  }
  return solve(A, b);
}
(async () => {
  const rotated = await sharp(src).rotate(270).toBuffer();
  const { data, info } = await sharp(rotated).extract(WIDE).raw().toBuffer({ resolveWithObject: true });
  const W = info.width, H = info.height, C = info.channels;
  const h = homography([[0, 0], [DW, 0], [DW, DH], [0, DH]], CORNERS);
  const outBuf = Buffer.alloc(DW * DH * 3);
  for (let v = 0; v < DH; v++) {
    for (let u = 0; u < DW; u++) {
      const d = h[6] * u + h[7] * v + 1;
      const x = (h[0] * u + h[1] * v + h[2]) / d;
      const y = (h[3] * u + h[4] * v + h[5]) / d;
      const x0 = Math.floor(x), y0 = Math.floor(y);
      const fx = x - x0, fy = y - y0;
      const o = (v * DW + u) * 3;
      if (x0 < 0 || y0 < 0 || x0 + 1 >= W || y0 + 1 >= H) { outBuf[o] = outBuf[o + 1] = outBuf[o + 2] = 0; continue; }
      for (let ch = 0; ch < 3; ch++) {
        const i00 = (y0 * W + x0) * C + ch, i10 = i00 + C, i01 = i00 + W * C, i11 = i01 + C;
        outBuf[o + ch] = (data[i00] * (1 - fx) + data[i10] * fx) * (1 - fy) + (data[i01] * (1 - fx) + data[i11] * fx) * fy;
      }
    }
  }
  await sharp(outBuf, { raw: { width: DW, height: DH, channels: 3 } }).png().toFile(out);
  console.log('Gerado:', path.relative(root, out), `${DW}x${DH}`);
})().catch((e) => { console.error(e); process.exit(1); });
