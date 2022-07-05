# myrrh-rs

Takes thorny JSON blobs and coverts them into a corresponding TypeScript type

Myrrh will squash common types so that types which are common have a single, shared type definition.

For example, take this JSON:

```json
{
  "payments": [
    {
      "amount": 1337,
      "currency": "USD"
    },
    {
      "amount": 420,
      "currency": "GBP"
    }
  ]
}
```

Myrr will generate the following TypeScript type definition:

```typescript
type DefaultType = {
  payments: DefaultType_0[];
};

type DefaultType_0 = { amount: number; currency: string };
```

With a common type which can be renamed by the user, this can be disabled using the `--squash false` flag.

## Usage

Run `cargo run --input ${path/to/input.json}` to receive the results to stdout.

To output to file, pass in an optional output file path with the flag `--output ${path/to/output.ts}`.

## Implementation details

The path I've chose to implement common type squashing is a Merkle Tree with a lookup table for common type detection and a type cache.

We build a n-ary Merkle tree which has nodes which have a hash which represents the name + type signatures of each of its descendent nodes, which is recalculated after the parsing has completed.

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

This structure means that as we are outputting the typescript type, we can check the hash against the lookup on each node, and if there is more than one node with the same hash and the type is not currently in the type output cache, we can generate a common type, adding it to a type cache.

At the end of the output process, we can then output common nodes will share a common type, this approach allows us to do this without traversing the entire tree at every step in the type generation process, while a performance penalty is incurred during the parsing process.
