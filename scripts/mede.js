// Mede o brilho médio das regiões de interesse numa foto do aparelho.
// Serve para detectar LED aceso sem depender do olho, comparando com uma referência.
// Uso: node scripts/mede.js <foto.jpg> [foto-referencia.jpg]
const sharp = require('./node_modules/sharp');

const REGIOES = {
  pads:       { left: 1000, top: 230, width: 800, height: 660 },
  strip:      { left: 100,  top: 430, width: 700, height: 60 },
  col_central:{ left: 830,  top: 230, width: 160, height: 660 },
  botoes_esq: { left: 180,  top: 600, width: 620, height: 300 },
  tela:       { left: 240,  top: 135, width: 250, height: 100 },
  // A parede ao fundo nunca muda. Serve para descontar a variação de exposição da webcam.
  parede:     { left: 60,   top: 60,  width: 200, height: 120 },
};

async function medir(arquivo) {
  const out = {};
  for (const [nome, r] of Object.entries(REGIOES)) {
    // O stats() do sharp lê a imagem de origem e ignora o extract encadeado,
    // então o recorte precisa virar um buffer antes de medir.
    const recorte = await sharp(arquivo).extract(r).png().toBuffer();
    const { channels } = await sharp(recorte).stats();
    out[nome] = {
      r: +channels[0].mean.toFixed(1),
      g: +channels[1].mean.toFixed(1),
      b: +channels[2].mean.toFixed(1),
    };
  }
  return out;
}

(async () => {
  const alvo = await medir(process.argv[2]);
  if (!process.argv[3]) {
    console.log(JSON.stringify(alvo, null, 1));
    return;
  }
  const ref = await medir(process.argv[3]);
  // Deriva da exposição, medida na parede, para descontar de todas as regiões.
  const deriva = { r: alvo.parede.r - ref.parede.r, g: alvo.parede.g - ref.parede.g, b: alvo.parede.b - ref.parede.b };
  console.log('região        ΔR     ΔG     ΔB   (já descontada a exposição)');
  for (const nome of Object.keys(REGIOES)) {
    if (nome === 'parede') continue;
    const d = ['r', 'g', 'b'].map((c) => alvo[nome][c] - ref[nome][c] - deriva[c]);
    const total = d.reduce((a, v) => a + Math.abs(v), 0);
    const txt = d.map((v) => v.toFixed(1).padStart(6)).join(' ');
    console.log(`${nome.padEnd(12)} ${txt}   ${total > 8 ? '<== MUDOU' : ''}`);
  }
  console.log(`(deriva de exposição na parede: ${deriva.r.toFixed(1)} ${deriva.g.toFixed(1)} ${deriva.b.toFixed(1)})`);
})();
