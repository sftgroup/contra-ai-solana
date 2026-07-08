# Contra AI Solana NFT — 部署 & 操作文档

> **最后更新**: 2026-07-09  
> **维护者**: stevenwang (team5-architect)

## 一、合约概览

| 项目 | 值 |
|------|-----|
| 程序 ID | `Gw8rwk9w8HNn8Emcgximggy9gtxxQaA7q6hHqboUT8aE` |
| 部署者 (Authority/Owner) | `HMnQrYxA4fJV8pX8NHK5LPeZxZAeUbpWBmQXpzeva9k9` |
| 程序数据地址 | `4ssdKm9gUYAvyvvJNG3yhMbp7BcnQE9thu8fKxHPsTTb` |
| 网络 | Solana Mainnet-Beta |
| GitHub | https://github.com/sftgroup/contra-ai-solana |
| 合约名称 | Contra AI NFT |

### 核心 PDA（程序派生地址）

| PDA | 地址 | 种子 | 说明 |
|-----|------|------|------|
| 状态 | `CQXj3HvNcSaQR88vfK5mrH75GiD6HaGng3zVLq3Q3uap` | `contra_state` | 存储所有配置（单账户） |
| 金库 | `FereG4vjpT1YBfYHRJWQNycXMfrrPYcf5mkPXmXaV88D` | `contra_treasury` | 收款中间账户 |
| Mint #N | 动态计算 | `contra_mint` + `token_id`(u64 LE) | 每个 NFT 的 Mint 地址 |

> Mint PDA 公计算规则：依次尝试 `bump` 255→0，找到第一个满足的地址。

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

---

## 十一、完整铸造操作指南

### 11.1 前置条件

| 条件 | 说明 |
|------|------|
| Node.js | 已安装，@solana/web3.js + @solana/spl-token |
| 部署者 Keypair | `/tmp/contra-keypair.json`（64 bytes） |
| SOL 余额 | 足够交易费（每次铸造成本约 0.000005 SOL） |
| USDC 余额 | 部署者 USDC 地址余额 ≥ mint_price |
| 合约状态 | `paused == false`，`total_minted < max_supply` |

### 11.2 铸造单个 NFT

```javascript
// /tmp/contra-mint/mint-one.js
const solana = require("@solana/web3.js");
const spl = require("@solana/spl-token");
const fs = require("fs");

// ===== 常量 =====
const PROGRAM_ID = new solana.PublicKey("Gw8rwk9w8HNn8Emcgximggy9gtxxQaA7q6hHqboUT8aE");
const DEPLOYER = new solana.PublicKey("HMnQrYxA4fJV8pX8NHK5LPeZxZAeUbpWBmQXpzeva9k9");
const USDC_MINT = new solana.PublicKey("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");

// PDA 派生
const [STATE_PDA] = solana.PublicKey.findProgramAddressSync(
  [Buffer.from("contra_state")], PROGRAM_ID
);
const [TREASURY_PDA] = solana.PublicKey.findProgramAddressSync(
  [Buffer.from("contra_treasury")], PROGRAM_ID
);
// Mint PDA: token_id = total_minted + 1
// 先读 state 获取 total_minted，计算 next_token_id

const connection = new solana.Connection(
  "https://solana-mainnet.g.alchemy.com/v2/HsbipfUQKw0a48ENgqlld",
  "processed"
);

async function mint() {
  const kp = solana.Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync("/tmp/contra-keypair.json", "utf8")))
  );

  // 1. 读取合约状态获取下一个 token_id
  const stateInfo = await connection.getAccountInfo(STATE_PDA);
  const stateData = Buffer.from(stateInfo.data);
  const totalMinted = Number(stateData.readBigUInt64LE(81)); // offset 81
  const nextTokenId = totalMinted + 1;
  console.log(`Total minted: ${totalMinted}, next: ${nextTokenId}`);

  // 2. 计算 Mint PDA
  const tid = Buffer.alloc(8);
  tid.writeBigUInt64LE(BigInt(nextTokenId));
  const [nftMintPDA] = solana.PublicKey.findProgramAddressSync(
    [Buffer.from("contra_mint"), tid], PROGRAM_ID
  );
  console.log(`NFT Mint PDA: ${nftMintPDA.toBase58()}`);

  // 3. 获取所有 ATA
  const payerAta = await spl.getAssociatedTokenAddress(USDC_MINT, DEPLOYER);
  const treasuryAta = await spl.getAssociatedTokenAddress(USDC_MINT, TREASURY_PDA, true);
  const beneficiaryAta = await spl.getAssociatedTokenAddress(USDC_MINT, DEPLOYER);
  const nftAta = await spl.getAssociatedTokenAddress(nftMintPDA, DEPLOYER);

  // 4. 构造 Instruction（15 个 accounts）
  const ix = new solana.TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: DEPLOYER, isSigner: true, isWritable: true },
      { pubkey: STATE_PDA, isSigner: false, isWritable: true },
      { pubkey: TREASURY_PDA, isSigner: false, isWritable: false },
      { pubkey: USDC_MINT, isSigner: false, isWritable: false },
      { pubkey: payerAta, isSigner: false, isWritable: true },
      { pubkey: treasuryAta, isSigner: false, isWritable: true },
      { pubkey: beneficiaryAta, isSigner: false, isWritable: true },
      { pubkey: nftMintPDA, isSigner: false, isWritable: true },
      { pubkey: nftAta, isSigner: false, isWritable: true },
      { pubkey: spl.TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: spl.ASSOCIATED_TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: solana.SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: DEPLOYER, isSigner: false, isWritable: false }, // beneficiary pubkey
      { pubkey: solana.SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: solana.SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
    ],
    data: Buffer.from([1]), // Mint instruction variant
  });

  // 5. 发送交易
  const bh = await connection.getLatestBlockhash("processed");
  const tx = new solana.Transaction({
    feePayer: DEPLOYER,
    recentBlockhash: bh.blockhash,
  }).add(ix);
  tx.sign(kp);

  const sig = await connection.sendRawTransaction(tx.serialize(), {
    skipPreflight: false,
  });
  console.log(`Sent: ${sig}`);

  // 6. 等待确认
  for (let i = 0; i < 30; i++) {
    await new Promise(r => setTimeout(r, 2000));
    const stat = await connection.getSignatureStatus(sig);
    if (
      stat?.value?.confirmationStatus === "confirmed" ||
      stat?.value?.confirmationStatus === "finalized"
    ) {
      console.log(`✅ Minted NFT #${nextTokenId}: ${nftMintPDA.toBase58()}`);
      return;
    }
    process.stdout.write(".");
  }
  console.log("❌ Timeout");
}

mint().catch(err => console.error(String(err).slice(0, 500)));
```

### 11.3 批量铸造

```javascript
// /tmp/contra-mint/batch-mint.js — 核心循环
async function batchMint(count) {
  const kp = solana.Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync("/tmp/contra-keypair.json", "utf8")))
  );

  for (let i = 0; i < count; i++) {
    // 每次先读状态获取 next_token_id
    const stateInfo = await connection.getAccountInfo(STATE_PDA);
    const totalMinted = Number(Buffer.from(stateInfo.data).readBigUInt64LE(81));
    const nextTokenId = totalMinted + 1;

    process.stdout.write(`Mint #${nextTokenId}... `);
    try {
      await mintOne(kp, nextTokenId); // mintOne 函数同上单次铸造逻辑
    } catch (e) {
      console.log(`ERROR: ${String(e).slice(0, 200)}`);
    }
    // 间隔 1 秒防止 RPC 限流
    await new Promise(r => setTimeout(r, 1000));
  }
}
```

### 11.4 修改 Mint Price（即时生效）

| 指令编号 | 17 (`SetMintPrice`) |
|----------|---------------------|

```javascript
const sol = require("@solana/web3.js");
const fs = require("fs");

const PROGRAM_ID = new sol.PublicKey("Gw8rwk9w8HNn8Emcgximggy9gtxxQaA7q6hHqboUT8aE");
const [STATE_PDA] = sol.PublicKey.findProgramAddressSync(
  [Buffer.from("contra_state")], PROGRAM_ID
);

async function setPrice(usdcAmount) {
  const kp = sol.Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync("/tmp/contra-keypair.json", "utf8")))
  );

  // 转换：USDC 有 6 位小数，0.01 USDC = 10000，10000 USDC = 10000000000
  const rawPrice = BigInt(usdcAmount * 1_000_000);
  console.log(`Setting price to ${usdcAmount} USDC (raw: ${rawPrice})`);

  const priceBuf = Buffer.alloc(8);
  priceBuf.writeBigUInt64LE(rawPrice);
  const data = Buffer.concat([Buffer.from([17]), priceBuf]);

  const ix = new sol.TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: kp.publicKey, isSigner: true, isWritable: false },
      { pubkey: STATE_PDA, isSigner: false, isWritable: true },
    ],
    data,
  });

  const conn = new sol.Connection(
    "https://solana-mainnet.g.alchemy.com/v2/HsbipfUQKw0a48ENgqlld",
    "processed"
  );
  const bh = await conn.getLatestBlockhash("processed");
  const tx = new sol.Transaction({ feePayer: kp.publicKey, recentBlockhash: bh.blockhash }).add(ix);
  tx.sign(kp);

  const sig = await conn.sendRawTransaction(tx.serialize(), { skipPreflight: false });
  console.log(`Sent: ${sig}`);

  for (let i = 0; i < 20; i++) {
    await new Promise(r => setTimeout(r, 2000));
    const s = await conn.getSignatureStatus(sig);
    if (s?.value?.confirmationStatus === "confirmed") {
      console.log("✅ Price updated!");
      return;
    }
  }
}

// 示例：设为 0.01 USDC
setPrice(0.01);

// 示例：设为 10000 USDC
setPrice(10000);
```

### 11.5 修改 Base URI（即时生效）

| 指令编号 | 18 (`SetBaseUri`) |
|----------|---------------------|

```javascript
async function setBaseUri(uri) {
  const kp = sol.Keypair.fromSecretKey(/* ... */);
  const uriBuf = Buffer.from(uri);
  const data = Buffer.concat([Buffer.from([18]), Buffer.from([uri.length]), uriBuf]);

  const ix = new sol.TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: kp.publicKey, isSigner: true, isWritable: false },
      { pubkey: STATE_PDA, isSigner: false, isWritable: true },
    ],
    data,
  });
  // ... 发送 tx
}
```

### 11.6 查询状态（读取链上配置）

```javascript
async function readState() {
  const connection = new sol.Connection(
    "https://solana-mainnet.g.alchemy.com/v2/HsbipfUQKw0a48ENgqlld",
    "confirmed"
  );
  const [STATE_PDA] = sol.PublicKey.findProgramAddressSync(
    [Buffer.from("contra_state")], PROGRAM_ID
  );

  const info = await connection.getAccountInfo(STATE_PDA);
  const d = Buffer.from(info.data);

  // Borsh 布局（参见 state.rs）
  const version = d[0];
  const authority = new sol.PublicKey(d.slice(1, 33));
  const paymentMint = new sol.PublicKey(d.slice(33, 65));
  const mintPrice = Number(d.readBigUInt64LE(65));
  const maxSupply = Number(d.readBigUInt64LE(73));
  const totalMinted = Number(d.readBigUInt64LE(81));
  const treasury = new sol.PublicKey(d.slice(89, 121));
  const beneficiary = new sol.PublicKey(d.slice(121, 153));
  const uriLen = d[281];
  const baseUri = d.slice(153, 153 + uriLen).toString("utf8");
  const paused = !!d[282];

  // Timelock fields (offset from 284)
  const pendingOwner = new sol.PublicKey(d.slice(284, 316));
  const pendingOwnerDeadline = Number(d.readBigInt64LE(316));
  // offset 324: pending_max_supply, 332: deadline
  // offset 340: pending_treasury, 372: deadline
  const pendingBeneficiary = new sol.PublicKey(d.slice(380, 412));
  const pendingBeneficiaryDeadline = Number(d.readBigInt64LE(412));

  console.log("=== Contra AI State ===");
  console.log(`Version: ${version}`);
  console.log(`Authority: ${authority.toBase58()}`);
  console.log(`Payment Mint: ${paymentMint.toBase58()}`);
  console.log(`Mint Price: ${mintPrice} (${mintPrice / 1e6} USDC)`);
  console.log(`Max Supply: ${maxSupply}`);
  console.log(`Total Minted: ${totalMinted}`);
  console.log(`Treasury: ${treasury.toBase58()}`);
  console.log(`Beneficiary: ${beneficiary.toBase58()}`);
  console.log(`Base URI: ${baseUri}`);
  console.log(`Paused: ${paused}`);

  if (pendingBeneficiary.toBase58() !== "11111111111111111111111111111111") {
    const now = Math.floor(Date.now() / 1000);
    const remaining = Math.max(0, pendingBeneficiaryDeadline - now);
    console.log(`\n⚠ Pending Beneficiary: ${pendingBeneficiary.toBase58()}`);
    console.log(`  Deadline: ${new Date(pendingBeneficiaryDeadline * 1000).toISOString()}`);
    console.log(`  Remaining: ${remaining}s (${(remaining / 3600).toFixed(1)}h)`);
    console.log(`  Expired: ${remaining === 0 ? "✅ YES" : "❌ NO"}`);
  }
}
```

### 11.7 转移 Beneficiary（24h Timelock）

**三步流程：Initiate → 等待 24h → Execute**

#### Step 1: Initiate（编号 13）

```javascript
async function initiateBeneficiaryChange(newBeneficiaryPubkey) {
  const kp = sol.Keypair.fromSecretKey(/* ... */);
  
  const newAddr = Buffer.from(new sol.PublicKey(newBeneficiaryPubkey).toBuffer());
  const data = Buffer.concat([Buffer.from([13]), newAddr]);

  const ix = new sol.TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: kp.publicKey, isSigner: true, isWritable: false },
      { pubkey: STATE_PDA, isSigner: false, isWritable: true },
      { pubkey: sol.SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
    ],
    data,
  });
  // ... 发送 tx
}
```

#### Step 2: 等待 24 小时（86400 秒）

```javascript
// 查询 deadline
async function checkTimelock() {
  const stateInfo = await connection.getAccountInfo(STATE_PDA);
  const d = Buffer.from(stateInfo.data);
  const deadline = Number(d.readBigInt64LE(412)); // pending_beneficiary_deadline
  const now = Math.floor(Date.now() / 1000);
  const remaining = deadline - now;
  if (remaining <= 0) {
    console.log("✅ Timelock expired, can execute now!");
  } else {
    console.log(`❌ ${remaining}s remaining (${(remaining/3600).toFixed(1)}h)`);
  }
}
```

#### Step 3: Execute（编号 15）

```javascript
async function executeBeneficiaryChange() {
  const kp = sol.Keypair.fromSecretKey(/* ... */);
  const data = Buffer.from([15]); // ExecuteBeneficiaryChange

  const ix = new sol.TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: kp.publicKey, isSigner: true, isWritable: false },
      { pubkey: STATE_PDA, isSigner: false, isWritable: true },
      { pubkey: sol.SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
    ],
    data,
  });
  // ... 发送 tx
}
```

### 11.8 转移 Owner（24h Timelock）

**四步流程：Initiate → 等待 24h → 切换为新 Owner → AcceptOwnership**

| 步骤 | 指令 | 编号 | 谁发起 |
|------|------|------|--------|
| Initiate | `InitiateOwnerTransfer` | 4 | 当前 Owner |
| Cancel | `CancelOwnerTransfer` | 5 | 当前 Owner |
| 等待 24h | — | — | — |
| Accept | `AcceptOwnership` | 6 | 新 Owner |

```javascript
// Initiate
const newOwner = new sol.PublicKey("9u9fPo4xunGR5o3DmEJUKSCPu61p4TYYxoyn9HfqEusH");
const data = Buffer.concat([Buffer.from([4]), newOwner.toBuffer()]);
// accounts: [authority(signer), state(w), clock]

// Cancel
const data = Buffer.from([5]);
// accounts: [authority(signer), state(w)]

// Accept (新 owner 用自己的 keypair 签名)
const data = Buffer.from([6]);
// accounts: [new_owner(signer), state(w), clock]
```

### 11.9 暂停/恢复铸造（即时生效）

| 指令编号 | 3 (`SetPause`) |
|----------|-----------------|

```javascript
async function setPause(paused) {
  const kp = sol.Keypair.fromSecretKey(/* ... */);
  const data = Buffer.from([3, paused ? 1 : 0]);

  const ix = new sol.TransactionInstruction({
    programId: PROGRAM_ID,
    keys: [
      { pubkey: kp.publicKey, isSigner: true, isWritable: false },
      { pubkey: STATE_PDA, isSigner: false, isWritable: true },
    ],
    data,
  });
  // ... 发送 tx
}
```

### 11.10 费用预估

| 操作 | SOL 费用 | USDC 费用 |
|------|----------|-----------|
| 铸造 1 个 NFT | ~0.00002 SOL | mint_price |
| 铸造 25 个 NFT | ~0.0005 SOL | 25 × mint_price |
| 修改 Price/URI/Pause | ~0.000005 SOL | 0 |
| Initiate Timelock | ~0.000005 SOL | 0 |
| Execute Timelock | ~0.000005 SOL | 0 |
| 升级合约 | ~0.03 SOL | 0 |
| 首次部署 | ~1.1 SOL | 0 |

---

## 十二、链上状态速查

```
2026-07-09 05:26 UTC+8

Authority:    HMnQrYxA4fJV8pX8NHK5LPeZxZAeUbpWBmQXpzeva9k9
Program:      Gw8rwk9w8HNn8Emcgximggy9gtxxQaA7q6hHqboUT8aE
State PDA:    CQXj3HvNcSaQR88vfK5mrH75GiD6HaGng3zVLq3Q3uap
Treasury PDA: FereG4vjpT1YBfYHRJWQNycXMfrrPYcf5mkPXmXaV88D

Mint Price:   10000 USDC (10000000000 raw)
Max Supply:   60
Total Minted: 26
Paused:       false
Base URI:     https://www.cai.codes/metadata/

Beneficiary:  HMnQrYxA4... (deployer)
Pending Ben:  9u9fPo4xun... (stevenwang) — 2026-07-09 20:12 UTC+8 expiry
Owner Pending: 9u9fPo4xun... — 2026-07-09 07:18 UTC+8 expiry

SOL Balance:  ~1.066 SOL
USDC Balance: ~1.75 USDC
```

---

*文档自动生成，请以链上状态为准。*
