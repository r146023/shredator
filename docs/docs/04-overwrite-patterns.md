# Overwrite Patterns

Shredator supports several overwrite patterns. Patterns control what bytes are written during each pass.

## Pattern list

| Pattern | Aliases | Behavior |
|---|---|---|
| `random` | none | Writes random bytes. Default. |
| `zeros` | `zero` | Writes `0x00`. |
| `ones` | `one` | Writes `0xFF`. |
| `alternating` | `alt` | Alternates between `0xAA` and `0x55` depending on pass number. |
| `dod` | none | Cycles zeros, ones, random. |
| `gutmann` | none | Uses a 35-pass Gutmann-style sequence. |

## Default pattern: random

```bash
shredator ./secret.txt --force
shredator ./secret.txt --force --pattern random
```

Random mode fills each write buffer with random bytes. In the updated implementation, random buffers are refilled per block instead of repeating one random buffer for the whole file.

## Zeros

```bash
shredator ./secret.txt --force --pattern zeros --passes 1
```

Writes `0x00` for every byte in each pass.

Good for:

- Fast cleanup where you want deterministic overwrite content.
- Testing with a hex editor.
- Benchmarking without random-generation overhead.

Not good for:

- Situations where you want varied overwrite content.

## Ones

```bash
shredator ./secret.txt --force --pattern ones --passes 1
```

Writes `0xFF` for every byte.

## Alternating

```bash
shredator ./secret.txt --force --pattern alt --passes 2
```

Pass behavior:

| Pass | Byte |
|---:|---:|
| odd | `0xAA` |
| even | `0x55` |

This gives a simple high/low bit alternation across passes.

## DoD-style pattern

```bash
shredator ./secret.txt --force --pattern dod --passes 3
shredator ./secret.txt --force --pattern dod --passes 6
```

Pass cycle:

| Pass modulo 3 | Byte pattern |
|---:|---|
| `1` | zeros |
| `2` | ones |
| `0` | random |

The implementation is DoD-style, not a formal certification of compliance with any particular erasure standard. Do not market this as certified secure erase.

## Gutmann-style pattern

```bash
shredator ./secret.txt --force --pattern gutmann
```

Selecting `gutmann` forces `passes = 35`.

Pass behavior:

- Passes `1..=4`: random.
- Passes `5..=8`: fixed legacy patterns.
- Passes `9..=31`: generated fixed pattern values.
- Passes `32..=35`: random.

This is a simplified Gutmann-style sequence. On modern storage, the practical benefit of Gutmann-style overwriting is usually less important than understanding the device/filesystem behavior.

## Choosing a pattern

### General use

```bash
shredator ./secret.txt --force --pattern random --passes 3
```

This is the default and is the most reasonable general-purpose mode.

### Fast deletion hygiene

```bash
shredator ./secret.txt --force --pattern zeros --passes 1
```

Use when speed matters more than multiple overwrite passes.

### Legacy multi-pass style

```bash
shredator ./secret.txt --force --pattern dod --passes 3
```

Good when you want deterministic multi-pass behavior and a final random pass.

### Avoid unnecessary theater

Do not assume more passes always produce meaningful extra protection. On SSDs and copy-on-write filesystems, repeated logical overwrites may not hit the same physical cells. For serious protection, prefer encryption before data is written and cryptographic erase when retiring media.

## Pass count and performance

Roughly:

```text
time ~= file_size * passes / write_throughput
```

A 10 GB file with 7 passes requires writing roughly 70 GB before truncation/deletion overhead.

## Benchmarking patterns

```bash
shredator ./test-1gb.bin --force --pattern zeros --passes 1 --benchmark
shredator ./test-1gb.bin --force --pattern random --passes 1 --benchmark
```

Random mode may be slower than fixed-byte mode because random data must be generated.
