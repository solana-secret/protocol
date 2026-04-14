# Solana SecretDAO Protocol Program Instruction Guide

## SecretDAO Program

### Tag = 0 — Mint & Initialize ATA
- **Purpose**: Create a user’s Associated Token Account (ATA) and mint ST / STA / STB after verifying the Mint Authority.
- **Data Structure**: `[tag:u8=0][amount:u64]`
- **Accounts**:
  1. `[Signer] Mint Authority`
  2. `[Writable] Mint`
  3. `[Writable] ATA`
  4. `[Readonly] ATA Owner`
  5. `[Readonly] SPL Token Program`
  6. `[Readonly] ATA Program`
  7. `[Readonly] System Program`
  8. `[Readonly] Rent Sysvar`

### Tag = 1 — Burn & Close ATA
- **Purpose**: Burn ST / STA / STB from the caller’s ATA and close the account automatically when the balance is zero.
- **Data Structure**: `[tag:u8=1][amount:u64]`
- **Accounts**:
  1. `[Signer] Token Owner`
  2. `[Writable] Owner ATA`
  3. `[Writable] Mint`
  4. `[Readonly] SPL Token Program`

### Tag = 2 — Burn With Encrypted Receipt
- **Purpose**: Optionally include an encrypted receiver payload in the burn flow so ST can be redeemed to a fresh address.
- **Data Structure**: `[tag:u8=2][amount:u64][has_receiver:u8][payload bytes?]`
- **Accounts**:
  1. `[Signer] Token Owner`
  2. `[Writable] Owner ATA`
  3. `[Writable] Mint`
  4. `[Writable] Redemption Address (receiver)`
  5. `[Readonly] SPL Token Program`

### Tag = 3 — Transfer & Close Sender
- **Purpose**: Transfer between two ATAs and close the sender’s ATA when its balance reaches zero to save rent.
- **Data Structure**: `[tag:u8=3][amount:u64]`
- **Accounts**:
  1. `[Signer] Sender`
  2. `[Writable] Sender ATA`
  3. `[Writable] Receiver ATA`
  4. `[Readonly] Receiver Owner`
  5. `[Readonly] Mint`
  6. `[Readonly] SPL Token Program`

### Tag = 4 — Batch Reward Mint
- **Purpose**: Mint rewards to multiple invitees in one transaction, with each recipient’s ATA prepared beforehand.
- **Data Structure**: `[tag:u8=4][reward_count:u8][amounts:u64[]]`
- **Accounts**:
  1. `[Signer] Mint Authority`
  2. `[Writable] Mint`
  3. `[Readonly] SPL Token Program`
  4. `[Readonly] ATA Program`
  5. `[Readonly] System Program`
  6. `[Readonly] Rent Sysvar`
  7. `[Variable] (owner, ata)×N`

---

## Merkle Program

### Tag = 0 — Initialize Merkle Root
- **Purpose**: Initialize and store the Merkle root, restricted to an authorized caller.
- **Data Structure**: `[tag:u8=0][root:bytes32]`
- **Accounts**:
  1. `[Writable] Merkle Root State`
  2. `[Signer] Authority`

### Tag = 1 — Update Leaf
- **Purpose**: Verify the old leaf and path, then write the new leaf and update the Merkle root.
- **Data Structure**: `[tag:u8=1][old_leaf:bytes32][new_leaf:bytes32][path_len:u8][siblings:bytes32[]][directions:u8[path_len]]`
- **Accounts**:
  1. `[Writable] Merkle Root State`
  2. `[Signer] Authority`

### Tag = 2 — Verify zk Proof
- **Purpose**: Concatenate the root/public inputs/proof, compare them with the verifier commitment, and perform lightweight verification.
- **Data Structure**: `[tag:u8=2][public_len:u8][public_bytes][proof_len:u8][proof_bytes]`
- **Accounts**:
  1. `[Readonly] Merkle Root State`
  2. `[Readonly] Verifier Commitment`

---

## Multisig Program

### Tag = 0 — Initialize Multisig
- **Purpose**: Configure up to five owners and a threshold, initializing the governance configuration account.
- **Data Structure**: `[tag:u8=0][owner_count:u8][threshold:u8][owners:Pubkey[owner_count]]`
- **Accounts**:
  1. `[Writable] Config Account`
  2. `[Signer] Authority`

### Tag = 1 — Propose Transfer
- **Purpose**: Create a record for a transfer proposal (destination, amount, nonce) and automatically count the proposer’s approval.
- **Data Structure**: `[tag:u8=1][destination:Pubkey][amount:u64]`
- **Accounts**:
  1. `[Writable] Config Account`
  2. `[Writable] Proposal Account`
  3. `[Signer] Proposer`

### Tag = 2 — Approve Proposal
- **Purpose**: Verify that the caller is a multisig owner and mark their approval bit.
- **Data Structure**: `[tag:u8=2]`
- **Accounts**:
  1. `[Readonly] Config Account`
  2. `[Writable] Proposal Account`
  3. `[Signer] Multisig Owner`

### Tag = 3 — Execute Transfer
- **Purpose**: Check the approval count, validate config/treasury/destination, execute the transfer, and mark the proposal as executed.
- **Data Structure**: `[tag:u8=3]`
- **Accounts**:
  1. `[Readonly] Config Account`
  2. `[Writable] Proposal Account`
  3. `[Writable] Treasury Account`
  4. `[Writable] Destination Account`
  5. `[Signer] Executor`

---

## Daovote

- **Key Points**:
  - Anyone in a collection can submit proposals and vote.
  - All proposals and votes are visible to everyone.
  - Participants can interpret votes however they choose.
  - Fully decentralized; proposals should include a URL pointing to the proposal document.

### Usage
```bash
npm run create -- <mint_id> <vote_id> <creator_0> <url>
npm run vote -- <mint_id> <vote_id> <creator_0> <vote_option>
```