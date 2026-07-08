# Contra AI Solana NFT — 部署 & 操作文档

## 一、合约概览

| 项目 | 值 |
|------|-----|
| 程序 ID | `Gw8rwk9w8HNn8Emcgximggy9gtxxQaA7q6hHqboUT8aE` |
| 部署者 (Authority) | `HMnQrYxA4fJV8pX8NHK5LPeZxZAeUbpWBmQXpzeva9k9` |
| 程序数据地址 | `4ssdKm9gUYAvyvvJNG3yhMbp7BcnQE9thu8fKxHPsTTb` |
| 网络 | Solana Mainnet-Beta |
| 合约名称 | Contra AI NFT |

### 核心 PDA

| PDA | 地址 | 种子 |
|-----|------|------|
| 状态 (State) | `CQXj3HvNcSaQR88vfK5mrH75GiD6HaGng3zVLq3Q3uap` | `contra_state` |
| 金库 (Treasury) | `FereG4vjpT1YBfYHRJWQNycXMfrrPYcf5mkPXmXaV88D` | `contra_treasury` |
| NFT Mint #1 | `GQCEDEhXboYXfxUWrYbWhvD6fDnsX9ERRrA2srC1gXcW` | `contra_mint` + `token_id`(LE) |

### 链上配置

| 参数 | 当前值 |
|------|--------|
| Owner | `HMnQrYxA4fJV8pX8NHK5LPeZxZAeUbpWBmQXpzeva9k9` |
| USDC Mint | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` |
| Mint Price | 10000（0.01 USDC） |
| Max Supply | 60 |
| Base URI | `https://www.cai.codes/metadata/` |
| Beneficiary | `HMnQrYxA4f...`（部署者，pending 24h timelock → `9u9fPo4xun...`） |
| Treasury | `FereG4vj...`（PDA） |

---

## 二、SBF 编译

### 环境要求

Solana Anza platform-tools v1.54（含 rustc 1.89.0-dev，支持 `edition2024`）。

```bash
# 下载并安装（如已安装跳过）
wget https://github.com/godmode-investments/platform-tools/releases/download/v1.54/platform-tools-linux-x86_64.tar.bz2
mkdir -p ~/.cache/solana/v1.54
tar -xjf platform-tools-linux-x86_64.tar.bz2 -C ~/.cache/solana/v1.54/

# 创建符号链接
rm -rf ~/.cache/solana/v1.48
ln -sf ~/.cache/solana/v1.54 ~/.cache/solana/v1.48
```

### 编译命令

```bash
cd ~/contra-ai-solana
export PATH="$HOME/.local/share/solana/install/active_release/bin:$PATH"
rm -rf target
cargo build-sbf --sbf-out-dir target/deploy
```

### 部署命令

```bash
# 升级部署（不换 program ID）
solana program deploy \
  target/deploy/contra_ai.so \
  --program-id Gw8rwk9w8HNn8Emcgximggy9gtxxQaA7q6hHqboUT8aE \
  --keypair /tmp/contra-keypair.json
```

部署费用：约 1.1 SOL（buffer rent，旧 buffer 自动回收后净花费约 0.03 SOL）

---

## 三、全部指令（Instruction）

合约共 **12 条指令**，通过 Borsh 编码：

| # | 指令 | 分类 | 描述 | Timelock |
|---|------|------|------|----------|
| 1 | `Initialize` | 初始化 | 创建 State PDA，设置初始配置 | 无 |
| 2 | `Mint` | 核心 | 支付 USDC → 铸 NFT，15 个 accounts | 无 |
| 3 | `SetPause` | 暂停 | 暂停/恢复铸造 | 无 |
| 4 | `InitiateOwnerTransfer` | Owner | 发起 Owner 转移 | ✅ 24h |
| 5 | `CancelOwnerTransfer` | Owner | 取消 Owner 转移 | 无 |
| 6 | `AcceptOwnership` | Owner | 新 Owner 确认（24h 后） | 无 |
| 7 | `InitiateMaxSupplyChange` | 参数 | 发起 MaxSupply 修改 | ✅ 24h |
| 8 | `CancelMaxSupplyChange` | 参数 | 取消 | 无 |
| 9 | `ExecuteMaxSupplyChange` | 参数 | 执行（24h 后） | 无 |
| 10 | `InitiateTreasuryChange` | Treasury | 发起 Treasury 修改 | ✅ 24h |
| 11 | `CancelTreasuryChange` | Treasury | 取消 | 无 |
| 12 | `ExecuteTreasuryChange` | Treasury | 执行（24h 后） | 无 |
| 13 | `InitiateBeneficiaryChange` | Beneficiary | 发起收益地址修改 | ✅ 24h |
| 14 | `CancelBeneficiaryChange` | Beneficiary | 取消 | 无 |
| 15 | `ExecuteBeneficiaryChange` | Beneficiary | 执行（24h 后） | 无 |
| 16 | `SetPaymentMint` | 参数 | 修改支付代币 | ❌ 即时 |
| 17 | `SetMintPrice` | 参数 | 修改铸造成本 | ❌ 即时 |
| 18 | `SetBaseUri` | 参数 | 修改 Metadata URI | ❌ 即时 |

### Timelock（24 小时时间锁）总结

| 参数 | Timelock | 说明 |
|------|----------|------|
| Owner | ✅ 24h | 三步：Initiate → 等待 24h → AcceptOwnership |
| Max Supply | ✅ 24h | 三步：Initiate → 等待 24h → Execute |
| Treasury | ✅ 24h | 三步：Initiate → 等待 24h → Execute |
| Beneficiary | ✅ 24h | 三步：Initiate → 等待 24h → Execute |
| Payment Mint | ❌ 即时 | 直接 SetPaymentMint |
| Mint Price | ❌ 即时 | 直接 SetMintPrice |
| Base URI | ❌ 即时 | 直接 SetBaseUri |
| Pause | ❌ 即时 | 直接 SetPause |

---

## 四、Mint 流程（JS）

### Mint 所需 Accounts（15 个）

```javascript
keys: [
  { pubkey: DEPLOYER, isSigner: true, isWritable: true },        // 0: payer
  { pubkey: STATE_PDA, isSigner: false, isWritable: true },      // 1: state
  { pubkey: TREASURY_PDA, isSigner: false, isWritable: false },  // 2: treasury_pda
  { pubkey: USDC_MINT, isSigner: false, isWritable: false },     // 3: payment_mint
  { pubkey: payerAta, isSigner: false, isWritable: true },       // 4: payer_token
  { pubkey: treasuryAta, isSigner: false, isWritable: true },    // 5: treasury_token
  { pubkey: beneficiaryAta, isSigner: false, isWritable: true }, // 6: beneficiary_token
  { pubkey: NFT_MINT_PDA, isSigner: false, isWritable: true },   // 7: nft_mint
  { pubkey: nftAta, isSigner: false, isWritable: true },         // 8: nft_token
  { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false }, // 9: token
  { pubkey: ASSOCIATED_TOKEN_PROGRAM_ID },                       // 10: ata_program
  { pubkey: SystemProgram.programId },                           // 11: system
  { pubkey: BENEFICIARY_PUBKEY, isSigner: false, isWritable: false }, // 12: beneficiary
  { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false }, // 13: rent
  { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false }, // 14: clock
]
```

### Mint 指令数据

```javascript
const mintData = Buffer.from([1]); // variant index for Mint
```

### 完整脚本

见 `/tmp/contra-mint/mint-final.js`

### Mint 内部流程

```
1. 校验 13 个 account（PDA、ATA、token program ID）
2. Transfer: payer_token → treasury_token (USDC × mint_price)
3. Forward: treasury_token → beneficiary_token (USDC × mint_price，treasury PDA 签名)
4. Create Account: 创建 NFT Mint PDA（82 bytes，owner=Token program）
5. Initialize Mint2: 初始化 Mint（decimals=0, authority = mint PDA）
6. Create ATA: 创建 payer 的 NFT ATA
7. Mint To: 铸 1 个 NFT 到 payer 的 ATA
8. 更新 State（total_minted++）
```

---

## 五、首次铸造结果

| 项目 | 值 |
|------|-----|
| NFT #1 Mint | `GQCEDEhXboYXfxUWrYbWhvD6fDnsX9ERRrA2srC1gXcW` |
| 交易签名 | `5tgXLgaQmuqMWw3619H1TwqXYrTqVmsPRdrRk5kMZ5CVTLQRPA...` |
| 确认 | ✅ Confirmed |

---

## 六、管理操作

### 修改 Mint Price（即时生效）

```bash
# 设为 0.01 USDC (10000)
node -e "
const sol = require('@solana/web3.js');
const kp = sol.Keypair.fromSecretKey(Uint8Array.from(JSON.parse(require('fs').readFileSync('/tmp/contra-keypair.json','utf8'))));
const prog = new sol.PublicKey('Gw8rwk9w8HNn8Emcgximggy9gtxxQaA7q6hHqboUT8aE');
const state = sol.PublicKey.findProgramAddressSync([Buffer.from('contra_state')], prog)[0];

const priceBuf = Buffer.alloc(8);
priceBuf.writeBigUInt64LE(10000n); // 0.01 USDC
const data = Buffer.concat([Buffer.from([17]), priceBuf]); // SetMintPrice = 17

const ix = new sol.TransactionInstruction({ programId: prog, keys: [
  { pubkey: kp.publicKey, isSigner: true, isWritable: false },
  { pubkey: state, isSigner: false, isWritable: true },
], data });
// ... send tx
"
```

### 修改 Base URI（即时生效）

```bash
node -e "
// SetBaseUri = 18
const uri = 'https://www.cai.codes/metadata/';
const uriLen = Buffer.from([uri.length]);
const data = Buffer.concat([Buffer.from([18]), uriLen, Buffer.from(uri)]);
// ... same as above
"
```

### 转移 Owner（24h Timelock）

```bash
# Step 1: Initiate
node -e "
const newOwner = new sol.PublicKey('9u9fPo4xunGR5o3DmEJUKSCPu61p4TYYxoyn9HfqEusH');
const data = Buffer.concat([Buffer.from([4]), newOwner.toBuffer()]); // InitiateOwnerTransfer=4
// accounts: [authority(signer), state(w), clock]
"

# Step 2: 等待 24 小时

# Step 3: AcceptOwnership (新 owner 发起)
# AcceptOwnership = 6
# accounts: [new_owner(signer), state(w), clock]
```

### 转移 Beneficiary（24h Timelock）

```bash
# Initiate
# InitiateBeneficiaryChange = 13

# 等待 24 小时

# Execute
# ExecuteBeneficiaryChange = 15
# accounts: [executor(signer), state(w), clock]
```

---

## 七、Bug 修复记录

### 2026-07-09: process_mint 6 个 bug 修复

| # | 位置 | 问题 | 修复 |
|---|------|------|------|
| 1 | create_treasury_ata_ix | invoke accounts 顺序全错，缺 wallet/mint | 加 `treasury_pda_info`、`payment_mint_info` |
| 2 | create_beneficiary_ata_ix | 传 `state_info` 当 beneficiary，缺 `payment_mint` | 用 `beneficiary_info`、`payment_mint_info` |
| 3 | forward_ix invoke_signed | signer 传 `state_info`，应该是 treasury_pda | 用 `treasury_pda_info.clone()` |
| 4 | mint_space = 165 | `Mint::LEN = 82`，`165` 是 `Account::LEN` | `spl_token::state::Mint::LEN` |
| 5 | init_mint accounts | `[nft_mint, nft_mint, token_program]` 全错 | `initialize_mint2` + `[nft_mint]` |
| 6 | mint_to invoke_signed | signer 传 `state_info`，应该是 mint_pda | 用 `nft_mint.clone()` |

所有修复已编译部署并成功铸造 NFT #1。

---

## 八、账户余额管理

**部署者**：`HMnQrYxA4fJV8pX8NHK5LPeZxZAeUbpWBmQXpzeva9k9`

- SOL：用于部署和交易费
- USDC：用于铸造 NFT（每个 0.01 USDC = 10000）
- Keypair：`/tmp/contra-keypair.json`

---

## 九、安全

- Authority 可以即时修改 Mint Price、Payment Mint、Base URI、Pause 状态
- Owner、Beneficiary、Treasury、Max Supply 有 24 小时时间锁保护
- 所有 PDA 的 seeds 都在合约中硬编码，无法被外部伪造
- 所有账户地址（ATA、PDA）在 `invoke` 前都经过 on-chain 验证
- 没有自毁、升级后门（deploy upgrade 是唯一升级方式）

## 十、铸造记录

### 2026-07-09 批量铸造 (#2 - #26)

全部 25 个铸造成功，共花 0.25 USDC。

| Token ID | Mint Address |
|----------|-------------|
| #2 | 3dVs6LCEjnjCRd2XLJUPyxcVgi8UhipQ54MpAytmbJyF |
| #3 | 8mLQzBgfynTdaHMFf2RJX9rtcUH1CsiCL3QvMpKGPDk |
| #4 | 863GDF2YFZya3dQD7dFw36AiQaenwdfsQyLe69spYq6o |
| #5 | CmoukrVUK3Q6p6NSJPqbnx6HYwVwNX7UmHTLpCb7vtFn |
| #6 | 9JHBcW7wCizQDEt3qaKh7zQF8nxgHWrrWXbru6x63Fji |
| #7 | EUsESfF9NAR2njFtKTjEDtPXbc1wKfPPvBims2oFKN2M |
| #8 | Db7aRCRged2KcHYqGrWrUb4gbqQuBz7M1c6i9bPUjTH5 |
| #9 | H8AVNp3Tzs8xGmvdH36bbuLzEF6jWXLH3YxksJySqSeU |
| #10 | HA8BWRhwdya9ToQ43nJBVoYpviqGbL1n1qpt39TgcTna |
| #11 | 8td4xwSEsX1uBo3FUXHnPUDJBoJo2y5pPVgU63YZocNB |
| #12 | 8D9EztXRhvcbxs9ca4WfVfixTeF2gMHmpp3NcciGk3r3 |
| #13 | EJC8gFaoark4ssTNgTLzEKGjVMMWREqqfPQ6ZS1zVy56 |
| #14 | DBYWAxu2fgT2ktRKrfPcwVmGPaBhuQAFGCBbLDxN87nx |
| #15 | EoZ3EgDn2smaicyMmsRgYqYNMpAJzffTadfsBgFko3v9 |
| #16 | BUtqVeyHGUqud4BzbxYNvpX7GfCmDWmCndZhcBn96qsS |
| #17 | kLGiCQ2BMca2YmEsHjJXnKAAehcKfRohuyKxv3PCS8k |
| #18 | EVLNZwtieiArR1T3TsVJvkKrhEpmQy23LswSZYCE3wV2 |
| #19 | 5zUQMH2tkv1K2KxdL6uYFWVmJsDFU6t8UgV5dXgiXqHp |
| #20 | 2af2dBNi912DXFSHCHABr1PydfCPFLiMuHtbgR5Q8jbR |
| #21 | 5tNvBmgBJC6seM8cAPEMoYehbP5abNWjvrx1nT9UNWgC |
| #22 | 6tXAwJvoaF3d8cLBLsqswnr8iX2WfQosAnRXtESPSNWS |
| #23 | 9aSaWAVGxWFaknBCZiX3LjjLyPnHDTvhnBboCJGK5EVK |
| #24 | 6p5nno1MiFzXNSTWqLi9XXE4ywioAauNG4PUzecnYzhz |
| #25 | 8MMAzMiMPrPiwiqTzR7UGhdsaWB6wPjkBNCKebwYBUF5 |
| #26 | 6ZQz4GeLoyecNgkvGtBYvDAWyDeMRrPo36TG7qLEEN8s |
