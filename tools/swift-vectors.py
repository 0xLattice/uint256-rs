#!/usr/bin/env python3
"""Generate arithmetic observations from unchanged pinned Swift UInt256."""
from pathlib import Path
import hashlib
import json
import subprocess
import tempfile
ROOT = Path(__file__).resolve().parents[1]
UPSTREAM = ROOT.parent / 'UInt256'
pin = json.loads((ROOT / 'upstream-lock.json').read_text())['UInt256']
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=UPSTREAM,text=True).strip()==pin
assert not subprocess.check_output(['git','status','--porcelain','--untracked-files=no'],cwd=UPSTREAM)
swift = r'''import Foundation
var values: [UInt256] = [0,1,2,3,UInt256.max]
for shift in [32,63,64,65,127,128,191,192,254,255] {
    let p = UInt256(1) << shift
    values.append(p); values.append(p - 1); values.append(p + 1)
}
var seed: UInt64 = 0x123456789abcdef
for _ in 0..<12 {
    var words: [UInt64] = []
    for _ in 0..<4 { seed = seed &* 6364136223846793005 &+ 1; words.append(seed) }
    values.append(UInt256(words))
}
var rows: [[String: Any]] = []
for (i,a) in values.enumerated() {
    for b in [values[(i+1)%values.count], values[(i*7)%values.count], UInt256(0), UInt256.max] {
        let add = a.addingReportingOverflow(b)
        let sub = a.subtractingReportingOverflow(b)
        let mul = a.multipliedReportingOverflow(by:b)
        let div = a.dividedReportingOverflow(by:b)
        let rem = a.remainderReportingOverflow(dividingBy:b)
        let full = a.multipliedFullWidth(by:b)
        var row: [String:Any] = ["a":a.toHexString(),"b":b.toHexString(),"add":add.partialValue.toHexString(),"addOverflow":add.overflow,"sub":sub.partialValue.toHexString(),"subOverflow":sub.overflow,"mul":mul.partialValue.toHexString(),"mulOverflow":mul.overflow,"div":div.partialValue.toHexString(),"divOverflow":div.overflow,"rem":rem.partialValue.toHexString(),"remOverflow":rem.overflow,"high":full.high.toHexString(),"low":full.low.toHexString(),"ones":a.nonzeroBitCount,"leading":a.leadingZeroBitCount,"trailing":a.trailingZeroBitCount,"swapped":a.byteSwapped.toHexString(),"and":(a&b).toHexString(),"or":(a|b).toHexString(),"xor":(a^b).toHexString()]
        row["shifts"] = [-1,0,1,63,64,65,127,128,191,192,255,256,257].map { n in ["n":n,"left":(a<<n).toHexString(),"right":(a>>n).toHexString()] as [String:Any] }
        if a > 0 {
            let hi = a >> 1
            let fullDiv = a.dividingFullWidth((high:hi,low:b))
            row["divHigh"] = hi.toHexString()
            row["fullQuotient"] = fullDiv.quotient.toHexString()
            row["fullRemainder"] = fullDiv.remainder.toHexString()
        }
        row["plainDiv"] = (a/b).toHexString()
        row["plainRem"] = (a%b).toHexString()
        rows.append(row)
    }
}
let data = try JSONSerialization.data(withJSONObject:rows,options:[.sortedKeys])
print(String(decoding:data,as:UTF8.self))
'''
with tempfile.TemporaryDirectory(prefix='uint256-swift-') as tmp:
    tmp = Path(tmp)
    (tmp/'main.swift').write_text(swift)
    sources = sorted((UPSTREAM/'Sources').glob('*.swift'))
    subprocess.run(['swiftc',*map(str,sources),str(tmp/'main.swift'),'-o',str(tmp/'oracle')],check=True)
    result = subprocess.check_output([str(tmp/'oracle')],text=True,timeout=120)
    rows = json.loads(result)
    path=ROOT/'tests/swift-vectors.json'
    path.write_text(json.dumps(rows,indent=2)+'\n')
    (ROOT/'tests/swift-provenance.json').write_text(json.dumps(dict(revision=pin,cases=len(rows),sha256=hashlib.sha256(path.read_bytes()).hexdigest(),sources={str(p.relative_to(UPSTREAM)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sources}),indent=2)+'\n')
    print(f'{len(rows)} arithmetic cases')
