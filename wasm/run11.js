const fs = require('fs');
const inst = new WebAssembly.Instance(new WebAssembly.Module(fs.readFileSync('/tmp/wasm1900/leptris.wasm')), {});
inst.exports.__stack_pointer.value = inst.exports.__heap_base;
const enc = new TextEncoder(), dec = new TextDecoder();
const mem = inst.exports.memory;
function allocB(b){const p=inst.exports.malloc(b.length+1);new Uint8Array(mem.buffer).set(b,p);new Uint8Array(mem.buffer)[p+b.length]=0;return p;}
function readC(p,bound){const v=new Uint8Array(mem.buffer);let e=p;while(v[e]!==0 && e-p<(bound||4096))e++;return dec.decode(v.subarray(p,e));}
const xml = '<root><item id="1">alpha</item><item id="2">beta</item></root>';
const x = allocB(enc.encode(xml));
const out = allocB(new Uint8Array(1024));
console.log('parse =>', inst.exports.stage_parse(x, xml.length));
const outP = out;  /* raw pointer */
for (const q of ['name(/*)', 'string(//item)', 'count(//item)', '1+1', 'avg(1 2 3)', 'concat(//item[1], "-", //item[2])']) {
  const qq = allocB(enc.encode(q));
  const n = inst.exports.stage_eval(qq);
  process.stdout.write('[' + q + '] n=' + n + ' string... ');
  const r = inst.exports.stage_string(outP, 1024);
  console.log(JSON.stringify(readC(outP)));
}
console.log('ALL ENGINE PATHS OK');
