# Contra AI Solana — 部署文档

## 链上信息

| 项目 | 值 |
|------|-----|
| 网络 | Solana Mainnet |
| 程序 ID | `Gw8rwk9w8HNn8Emcgximggy9gtxxQaA7q6hHqboUT8aE` |
| BPF Loader | `BPFLoaderUpgradeab1e11111111111111111111111` |
| ProgramData | `4ssdKm9gUYAvyvvJNG3yhMbp7BcnQE9thu8fKxHPsTTb` |
| Authority（部署者） | `HMnQrYxA4fJV8pX8NHK5LPeZxZAeUbpWBmQXpzeva9k9` |

## 合约参数

| 参数 | 值 |
|------|-----|
| payment_mint (USDC) | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` |
| mint_price | 10,000 USDC (= 10000000000 raw) |
| max_supply | 60 |
| base_uri | `https://www.cai.codes/metadata/` |
| beneficiary | `9u9fPo4xunGR5o3DmEJUKSCPu61p4TYYxoyn9HfqEusH` |
| owner | `9u9fPo4xunGR5o3DmEJUKSCPu61p4TYYxoyn9HfqEusH` |

## 部署密钥

| 项目 | 值 |
|------|-----|
| 部署者地址 | `HMnQrYxA4fJV8pX8NHK5LPeZxZAeUbpWBmQXpzeva9k9` |
| Keypair 文件 | `/tmp/contra-keypair.json`（64 bytes） |

## 编译环境

### SBF 编译问题与解决方案

**问题**：Agave 2.3.3 内嵌 `cargo-build-sbf` 使用 platform-tools v1.48（rustc 1.84.1），不支持 `edition2024`。solana-program 的传递依赖（zerocopy 0.8.x、toml_datetime、crypto-common 等）需要 rustc 1.85+。

```
error: feature `edition2024` is required
  The package requires the Cargo feature called `edition2024`,
  but that feature is not stabilized in this version of Cargo (1.84.0)
```

**解决方案**：使用 Anza platform-tools v1.54（rustc 1.89.0-dev）

```bash
# 1. 下载新版 platform-tools
curl -sL "https://github.com/anza-xyz/platform-tools/releases/download/v1.54/platform-tools-linux-x86_64.tar.bz2" \
  -o /tmp/pt-v1.54.tar.bz2

# 2. 解压到 cache 目录
mkdir -p ~/.cache/solana/v1.54
tar xjf /tmp/pt-v1.54.tar.bz2 -C ~/.cache/solana/v1.54/
# 调整目录结构
mkdir -p ~/.cache/solana/v1.54/platform-tools
mv ~/.cache/solana/v1.54/rust ~/.cache/solana/v1.54/llvm \
  ~/.cache/solana/v1.54/platform-tools/

# 3. 替换 cargo-build-sbf 使用的 platform-tools
rm -rf ~/.cache/solana/v1.48
ln -s v1.54 ~/.cache/solana/v1.48

# 4. 编译
cargo build-sbf --sbf-out-dir target/deploy
```

## 部署命令

### 升级部署（替换已有程序）

```bash
solana program deploy \
  target/deploy/contra_ai.so \
  --program-id Gw8rwk9w8HNn8Emcgximggy9gtxxQaA7q6hHqboUT8aE \
  --keypair /tmp/contra-keypair.json
```

> ⚠️ 升级部署需要 ~1.068 SOL（新程序 buffer rent + gas）。部署完成后 buffer rent 会退还。

### 检查程序状态

```bash
solana program show Gw8rwk9w8HNn8Emcgximggy9gtxxQaA7q6hHqboUT8aE
```

## 合约改动摘要（v2 → 当前）

| 改动 | 说明 |
|------|------|
| `Initialize` 独立 beneficiary | beneficiary 不再是默认=payer，作为独立参数传入 |
| Mint 自动创建 ATA | `process_mint` 自动创建 treasury PDA 的 ATA 和 beneficiary 的 ATA（如不存在） |
| 取消 24h timelock | `SetMintPrice` / `SetPaymentMint` 改为直接 instant 修改（authority-only） |
| P0/P1 安全审计修复 | Account validation 增强 + timelock 逻辑修正 |

## 指令列表（12 条）

1. `Initialize` — 初始化合约，设置 payment_mint、mint_price、max_supply、base_uri、beneficiary
2. `MintNFT` — 铸造 NFT，自动创建所需 ATA
3. `SetPaymentMint` — 修改支付代币（即时生效）
4. `SetMintPrice` — 修改铸造价格（即时生效）
5. `SetBaseUri` — 修改 baseURI
6. `SetMaxSupply` — 修改最大供应量
7. `SetBeneficiary` — 修改受益人地址
8. `WithdrawFunds` — 提取合约中的 USDC
9. `Pause` — 暂停铸造
10. `Unpause` — 恢复铸造
11. `TransferOwnership` — 转移所有权
12. `AcceptOwnership` — 接受所有权

## 依赖版本

```toml
[dependencies]
solana-program = "2.3.0"
spl-token = "7.0"
spl-associated-token-account = { version = "6.0", features = ["no-entrypoint"] }
borsh = "1.5"
borsh-derive = "1.5"
thiserror = "2.0"
```

## 源码仓库

- **GitHub**: https://github.com/sftgroup/contra-ai-solana
- **分支**: master

---

## 部署历史

### 2026-07-09 — 升级部署 v3

| 项目 | 值 |
|------|-----|
| 交易签名 | `5rXfLkPrx6qZndxYvGT3p8WZLYhHz8u2FWAkvrsuXuAkmghQdEpXpLHXwRJrLBYQshVo1cW8qkuNqWTjeSBsTwdU` |
| Slot | 431,655,386 |
| 程序大小 | 153,200 bytes |
| 改动 | beneficiary 独立参数、Mint 自动创建 ATA、取消 timelock、安全审计修复 |
| 编译环境 | platform-tools v1.54 (rustc 1.89.0-dev) |
| 花费 | ~0.025 SOL |
