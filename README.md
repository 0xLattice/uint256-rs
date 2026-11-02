# lattice-uint256

Reusable Rust arithmetic port of `0xLattice/UInt256`, pinned in
`upstream-lock.json`. Separate unpublished repository, branch
`codex/rust-platform-uint256`. Upstream MIT attribution is retained.

`UInt256` stores 32 bytes in big-endian order and provides arithmetic, bitwise
operators, comparisons, hashes, shifts, byte order conversion, bit counts,
radix parsing/formatting, overflow reporting, checked operations, explicit
wrapping/saturating operations, and full-width multiply/divide.

```rust
use lattice_uint256::UInt256;
let a = UInt256::from(42u64);
assert_eq!(a.checked_mul(UInt256::from(2)), Some(UInt256::from(84)));
assert_eq!(UInt256::MAX.checked_add(UInt256::from(1)), None);
```

Compatibility details matter:

- Ordinary addition/subtraction/multiplication wrap, including in debug builds.
- Ordinary division/remainder by zero return zero, matching Swift. Reporting
  methods return the unchanged dividend with `overflow = true`; checked methods
  return `None`. Use checked methods for admission or input validation.
- Shifts by 256 or more, and the explicit signed shift methods with a negative
  count, return zero. Bit counts of zero are 256.
- Display and `to_hex_string()` use 64 lowercase hex digits, matching Swift's
  description. `to_string_radix(10)` gives decimal; `FromStr` parses decimal.
- `from_swift_words` is the source array initializer: pad short input on the left,
  and retain the *first* four words of long input. `from_i64_wrapping` reproduces
  sign extension of Swift's concrete Int initializer.
- `full_div` refuses division by zero and a quotient wider than 256 bits. Swift's
  source does not provide a useful checked error contract for these invalid
  full-width operands. Valid operands are compared directly below.

Arithmetic uses `num-bigint` internally. This is not a constant-time secret-key
arithmetic implementation, and does not claim the performance of the source's
Karatsuba/Barrett routines. The protocol's `lattice-primitives::UInt256` keeps its
existing consensus serializer; convert explicitly through its public bytes.
No consensus wire-format change is made by adding this library.

## Validation

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
python3.12 tools/swift-vectors.py
```

The generator compiles the unchanged pinned Swift sources. It covers 188 pairs
across zero/max, limb boundaries, neighboring powers of two, and deterministic
wide random values. Each row compares all five reporting operations, ordinary
division/remainder, full-width multiplication, valid full-width division,
bitwise operations, byte reversal and bit counts. Thirteen signed shift counts
per row include negative, limb-boundary and oversized counts. Source hashes and
corpus hash are recorded beside the checked-in vectors.

Rust additionally checks overflow refusals, invalid divisors/radices, endian
round trips and source array conversion. Finite comparisons are not a proof of
all arithmetic inputs. Generic Swift protocol conversions (including its
UInt64-narrowing behavior for generic BinaryInteger/floating inputs), Swift's
random-number convenience APIs, and algorithmic performance parity are not
claimed by these Rust APIs. There is no separate lattice-formal UInt256 model;
existing formal work/ASERT checks apply to the protocol implementation.
