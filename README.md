# myrrh-rs

Takes thorny JSON blobs and coverts them into a corresponding TypeScript type

## Usage

Run `cargo run --input ${path/to/input.json}` to receive the results to stdout.

To output to file, pass in an optional output file path with the flag `--output ${path/to/output.ts}`.

## Implementation details

Myrrh currently at time of writing performs three passes of the JSON structure, the first pass is to parse using serde, then we parse the serde output into internal Myrrh data structures, then finally as part of the output process we do a third pass.

## Future Hopes

Currently, Myrrh cannot simplify common types, this means that the TypeScript types that are generated can be large, for example:

Given an input file:

`input.json`

```
{
    "paymentOne": {
        "amount": 1337
        "status": "paid"
    },
    "paymentTwo": {
        "amount": 420
        "status": "unpaid"
    }
}
```

Myrrh will generate:

```
type DefaultType = {
    "paymentOne": {
        "amount": number
        "status": string
    },
    "paymentTwo": {
        "amount": number
        "status": string
    }
}
```

Where we'd actually much prefer Myrrh to generate:

```
type CommonTypeOne = {
    "amount": number
    "status": string
}

type DefaultType = {
    "paymentOne": CommonTypeOne
    "paymentTwo": CommonTypeOne
}
```

The path I've chose to implement to achieve this is a Merkle Tree with a lookup table for common type detection and a type cache.

We'll build a n-ary Merkle tree which has nodes which have a hash which represents the name + type signatures of each of its descendent nodes, which is recalculated every time a new node is added.

Diagram:

```
┌────────────────────────────────┐
│Node                            │
├────────────────────────────────┤
│Type: String                    │
│Name: root                      │
│Hash: 3child                    │
│      +2grandchildren           ◄────────────┐
│                                │            │
│                                │            │
│                                │            │
└────────▲──────────────▲────────┘            │
         │              │                     │
         │              │                     │
         │              │                     │
         │              │                     │
         │              │                     │
┌────────┴──────┐   ┌───┴───────────┐    ┌────┴──────────┐
│Node           │   │Node           │    │Node           │
├───────────────┤   ├───────────────┤    ├───────────────┤
│Type: String   │   │Type: String   │    │Type: String   │
│Name: test     │   │Name: test     │    │Name: test     │
│Hash: 2child   │   │Hash: test     │    │Hash: test     │
└───────▲─────▲─┘   └───────────────┘    └───────────────┘
        │     │
        │     └──────────────┐
        │                    │
┌───────┴───────┐    ┌───────┴───────┐
│Node           │    │Node           │
├───────────────┤    ├───────────────┤
│Type: String   │    │Type: String   │
│Name: test     │    │Name: test     │
│Hash: test     │    │Hash: test     │
└───────────────┘    └───────────────┘
```

As a node is added at the root of the tree, the hash of each node with an upwards dependency on its value (including the root node) is recalculated and the node is added to the lookup table. As hashes are recomputed, the lookup table is also purged, with the old hash decremented and new hash added.

This structure means that as we are outputting the typescript type, we can check the hash against the lookup on each node, and if there is more than one node with the same hash and the type is not currently in the type output cache, we can generate a common type, adding it to a type cache.

At the end of the output process, we can then output common nodes will share a common type, this approach allows us to do this without traversing the entire tree at every step in the type generation process, while a performance penalty is incurred during the parsing process.
