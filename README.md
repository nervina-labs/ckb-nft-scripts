# ckb-nft-scripts

[![License](https://img.shields.io/badge/license-MIT-green)](https://github.com/nervina-labs/ckb-nft-scripts/blob/develop/COPYING)
[![Github Actions CI](https://github.com/nervina-labs/ckb-nft-scripts/workflows/CI/badge.svg?branch=develop)](https://github.com/nervina-labs/ckb-nft-scripts/actions)

The NFT Type Scripts implement of [RFC: Multi-purpose NFT Draft Spec](https://talk.nervos.org/t/rfc-multi-purpose-nft-draft-spec/5434) on [Nervos CKB](https://www.nervos.org/).

## Pre-requirement

- [capsule](https://github.com/nervosnetwork/capsule) >= 0.4.3
- [ckb-cli](https://github.com/nervosnetwork/ckb-cli) >= 0.35.0

> Note: Capsule uses docker to build contracts and run tests. https://docs.docker.com/get-docker/
> and docker and ckb-cli must be accessible in the PATH in order for them to be used by Capsule.

## Getting Started

Build contracts:

```sh
capsule build
```

Run tests:

```sh
capsule test
```

## Deployment

### 1. Update the deployment configurations

Update the `deployment.toml` referring to the [Capsule Docs](https://docs.nervos.org/docs/labs/sudtbycapsule#deploy)

### 2. Build release version of the script

The release version of script doesn’t include debug symbols which makes the size smaller.

```sh
capsule build --release
```

### 3. Deploy the scripts

```sh
capsule deploy --address <ckt1....> --fee 0.001
```

If the `ckb-cli` has been installed and `dev-chain` RPC is connectable, you will see the deployment plan:

```
Deployment plan:
---
migrated_capacity: 25798.0 (CKB)
new_occupied_capacity: 184467436505.09551616 (CKB)
txs_fee_capacity: 0.001 (CKB)
total_occupied_capacity: 21566.0 (CKB)
recipe:
  cells:
    - name: nft-type
      index: 0
      tx_hash: "0xa105c3277ea36914e2af26e749adb307276f89f614dc945f9f44988b4be9c1d6"
      occupied_capacity: 21566.0 (CKB)
      data_hash: "0x2123504d48d69e6e4f5e749dcb551fb5dfe32af027daa35fcdbfc61a67bf9859"
      type_id: "0xb1837b5ad01a88558731953062d1f5cb547adf89ece01e8934a9f0aeed2d959f"
  dep_groups: []
Confirm deployment? (Yes/No)
Yes
Password:
(1/1) Sending tx a105c3277ea36914e2af26e749adb307276f89f614dc945f9f44988b4be9c1d6
Deployment complete
```
