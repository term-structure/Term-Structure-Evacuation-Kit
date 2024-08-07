# Term Structure Evacuation Kit

The Term Structure Evacuation Kit is a tool designed for managing term structure data and generating zero-knowledge proofs for data evacuation. Below are the instructions to build, install, and use the kit.

## Building and Installing

To build the project and install the tool:

```bash
cargo build --release
cargo install --path term-structure-evacuation-kit
```

## Usage

After installation, you can use the `ts-evacu` command with various subcommands to manage and process your data. First, copy the example configuration file and replace it with your own API key:

```bash
cp config.json.example config.json
```

### Update State

To update the state based on the provided configuration file and end block ID, use the `update_state` command. If the end block ID (`-e`) is not specified, the state will be updated to the latest block:

```bash
ts-evacu update_state -c config.json -e 19968461
```

### Query Balance

To query the balance of a specific account for a specified asset, use the `query` command with the account ID and token ID:

```bash
ts-evacu query -c config.json -a 2 -t 2
```

### Export Input Files

To export the input files required for the evacuation zk proof, use the `export` command with the account ID and token ID:

```bash
ts-evacu export -c config.json -a 2 -t 2 > ./input.json
```

### Consume Data

To export the data required to consume L1 requests in the smart contract, use the `consume` command with the configuration file:

```bash
ts-evacu consume -c config.json
```

## Generating Zero-Knowledge Proofs

Download the [zkTrue-up Evacuation Witness Calculator](https://storage.googleapis.com/trusted-setup.v1.zktrue-up.ts.finance/zkTrue-up%20Evacuation%20Witness%20Calculator.zip) and the [zkTrue-up Evacuation Zkey](https://storage.googleapis.com/trusted-setup.v1.zktrue-up.ts.finance/evacu_finalized.zkey).

Generate the witness:

```bash
node zkTrueUp-Evacuation_js/generate_witness.js zkTrueUp-Evacuation_js/zkTrueUp-Evacuation.wasm ./input.json ./witness.wtns
```

Prove the circuit using `snarkjs`:

```bash
npx snarkjs groth16 prove evacu_finalized.zkey ./witness.wtns ./proof.json ./public.json
```

Export the Solidity calldata:

```bash
npx snarkjs zkey export soliditycalldata ./public.json ./proof.json
```

## License

[MIT](LICENSE)

---

For more detailed usage and options, please refer to the help command `ts-evacu --help`.