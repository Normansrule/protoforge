// Regenerates docs/references.md and docs/reference-repos.md from the data
// that drives the Guide & references page, so the docs and the app agree.
//   node --experimental-strip-types web/src/pages/learn/gen-docs.mjs   (Node 22+)
import { REFERENCES } from './references.ts';
import { REPOS } from './repos.ts';
import { RESOURCES } from './resources.ts';
import { GLOSSARY } from './glossary.ts';
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('../../../../', import.meta.url));
const EXTRA = { BSIM: 'Berkeley short-channel insulated-gate field-effect transistor model', LU: 'lower–upper (decomposition)', EDA: 'electronic design automation', DSN: 'Specctra design file', SAC: 'soft actor–critic', IEEE: 'Institute of Electrical and Electronics Engineers', ISO: 'International Organization for Standardization', ASTM: 'ASTM International (formerly American Society for Testing and Materials)', ASME: 'American Society of Mechanical Engineers', AIAG: 'Automotive Industry Action Group', VDA: 'Verband der Automobilindustrie (German Association of the Automotive Industry)', MIT: 'Massachusetts Institute of Technology', GDSII: 'Graphic Design System II', MARG: 'magnetic, angular rate and gravity', SKY130: 'SkyWater 130 nm process', HDL: 'hardware description language', CNC: 'computer numerical control', W3C: 'World Wide Web Consortium', ISCAS: 'International Symposium on Circuits and Systems', ICORR: 'International Conference on Rehabilitation Robotics', ICIP: 'International Conference on Image Processing', IDETC: 'International Design Engineering Technical Conferences', CIE: 'Computers and Information in Engineering', ERL: 'Electronics Research Laboratory', TAC: 'Transactions on Automatic Control', SIAM: 'Society for Industrial and Applied Mathematics', AIEE: 'American Institute of Electrical Engineers', ASSP: 'Acoustics, Speech, and Signal Processing', CMU: 'Carnegie Mellon University', AFCRC: 'Air Force Cambridge Research Center', SMC: 'Systems, Man, and Cybernetics', IRE: 'Institute of Radio Engineers', MDP: 'Markov decision process' };
function abbreviations(text) {
  const map = new Map();
  for (const t of GLOSSARY) for (const k of t.term.split(/\s*\/\s*/)) if (/^[A-Z0-9²]{2,}$/.test(k)) map.set(k, t.full);
  for (const [k, v] of Object.entries(EXTRA)) map.set(k, v);
  const found = [...new Set(text.match(/\b[A-Z][A-Z0-9]{1,6}\b/g) ?? [])].filter((k) => map.has(k)).sort();
  return `**Abbreviations:** ${found.map((k) => `${k} — ${map.get(k)}`).join('; ')}.\n`;
}
const labNames = { systems: 'Systems Studio', circuit: 'Circuit Lab', power: 'Power & Thermal', pcb: 'PCB Studio', dsp: 'DSP Lab', chip: 'Chip Lab', control: 'Control Lab', robotics: 'Robotics Lab', rl: 'RL Lab', optics: 'Optics Lab', cad: 'CAD Studio', print: 'Print Lab', molding: 'Molding Lab', mfg: 'Manufacturing Hub', structures: 'Structures Lab', quantum: 'Quantum Lab', proto: 'Prototype & Firmware', build: 'Build Guide', program: 'Program & Business Lab', reliability: 'Reliability & Test Lab', ergonomics: 'Human Factors Lab', mechanisms: 'Mechanisms Lab', fluids: 'Thermal & Fluids Lab', rf: 'RF & Antenna Lab', versions: 'Versions', toolchain: 'Web toolchain' };
let md = `# References\n\nThe consolidated bibliography of Protoforge, gathered from every lab guide (\`docs/labs/*.md\`) and the engine's doc comments, deduplicated and grouped by discipline. The same list is shown on the **Guide & references** page of the app (\`#/learn\`), generated from \`web/src/pages/learn/references.ts\` — edit that file and regenerate this one with \`node --experimental-strip-types web/src/pages/learn/gen-docs.mjs\` (Node 22) so the two stay in sync.\n\nLinks are given only where a source is freely available online. The labs that cite each work are listed in brackets.\n`;
let n = 0;
for (const g of REFERENCES) {
  md += `\n## ${g.discipline}\n\n`;
  for (const r of g.refs) {
    n++;
    md += `${n}. ${r.text}${r.href ? ` <${r.href}>` : ''} [${r.labs.map((l) => labNames[l] ?? l).join(', ')}]\n`;
  }
}
md += `\n## Learning resources (English)\n\nFree books and courses first, then the textbooks the labs cite most.\n\n| Resource | Author / publisher | Kind | Why |\n|---|---|---|---|\n`;
const order = ['Free book', 'Free course', 'Practice', 'Handbook', 'Textbook'];
for (const r of [...RESOURCES].sort((a, b) => order.indexOf(a.kind) - order.indexOf(b.kind))) {
  md += `| ${r.url ? `[${r.title}](${r.url})` : r.title} | ${r.who} | ${r.kind} | ${r.why} |\n`;
}
md += `\nSee also [reference-repos.md](reference-repos.md) for open-source projects to study.\n`;
md = md.replace(/\n## /, `\n${abbreviations(md)}\n## `);
writeFileSync(`${root}docs/references.md`, md);

let rp = `# Reference repositories\n\nMature open-source projects that do what each Protoforge lab teaches, at professional scale. Study their code, cross-check Protoforge's numbers against them, and graduate your design to them. Contributors: when you extend a lab, read the matching project first and cite it.\n\nThe same list appears on the **Guide & references** page (\`#/learn\`), generated from \`web/src/pages/learn/repos.ts\` (regenerate with \`node --experimental-strip-types web/src/pages/learn/gen-docs.mjs\`). GitHub \`owner/name\` is given where the project lives on GitHub; otherwise the official site.\n`;
for (const g of REPOS) {
  rp += `\n## ${g.discipline}\n\n| Project | Where | What to learn from it |\n|---|---|---|\n`;
  for (const r of g.items) {
    const where = r.repo ? `[\`${r.repo}\`](https://github.com/${r.repo})${r.url ? ` · <${r.url}>` : ''}` : r.url ? `<${r.url}>` : '—';
    rp += `| **${r.name}** | ${where} | ${r.learn.replace(/\|/g, '\\|')} |\n`;
  }
}
rp = rp.replace(/\n## /, `\n${abbreviations(rp)}\n## `);
writeFileSync(`${root}docs/reference-repos.md`, rp);
console.log('refs', n);
