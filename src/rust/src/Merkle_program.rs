use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    hash::hashv,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};

entrypoint!(process);

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (tag, rest) = data
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    match tag {
        0 => initialize_root(program_id, accounts, rest),
        1 => update_leaf(program_id, accounts, rest),
        2 => verify_zk_proof(program_id, accounts, rest),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

fn initialize_root(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() != 32 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let accounts_iter = &mut accounts.iter();
    let state_account = next_account_info(accounts_iter)?;
    let authority = next_account_info(accounts_iter)?;

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if state_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    if state_account.data_len() < 32 {
        return Err(ProgramError::InvalidAccountData);
    }

    let mut data_ref = state_account.data.borrow_mut();
    data_ref[..32].copy_from_slice(data);
    msg!("Merkle root initialized");
    Ok(())
}

fn update_leaf(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() < 65 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let accounts_iter = &mut accounts.iter();
    let state_account = next_account_info(accounts_iter)?;
    let authority = next_account_info(accounts_iter)?;

    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if state_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    if state_account.data_len() < 32 {
        return Err(ProgramError::InvalidAccountData);
    }

    let mut offset = 0;
    let mut old_leaf = [0u8; 32];
    old_leaf.copy_from_slice(&data[offset..offset + 32]);
    offset += 32;

    let mut new_leaf = [0u8; 32];
    new_leaf.copy_from_slice(&data[offset..offset + 32]);
    offset += 32;

    let path_len = data[offset] as usize;
    offset += 1;

    if path_len == 0 || path_len > 32 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let nodes_end = offset + path_len * 32;
    if data.len() < nodes_end + path_len {
        return Err(ProgramError::InvalidInstructionData);
    }

    let mut path_nodes = Vec::with_capacity(path_len);
    for i in 0..path_len {
        let start = offset + i * 32;
        let mut node = [0u8; 32];
        node.copy_from_slice(&data[start..start + 32]);
        path_nodes.push(node);
    }

    offset = nodes_end;
    let directions = &data[offset..offset + path_len];

    let stored_root = {
        let state_data = state_account.data.borrow();
        let mut root = [0u8; 32];
        root.copy_from_slice(&state_data[..32]);
        root
    };

    let current_root = compute_merkle_root(&old_leaf, &path_nodes, directions);
    if current_root != stored_root {
        return Err(ProgramError::InvalidAccountData);
    }

    let updated_root = compute_merkle_root(&new_leaf, &path_nodes, directions);
    state_account
        .data
        .borrow_mut()[..32]
        .copy_from_slice(&updated_root);

    msg!("Merkle root updated");
    Ok(())
}

fn verify_zk_proof(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if data.len() < 2 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let accounts_iter = &mut accounts.iter();
    let root_account = next_account_info(accounts_iter)?;
    let verifier_account = next_account_info(accounts_iter)?;

    if root_account.owner != program_id || verifier_account.owner != program_id {
        return Err(ProgramError::IncorrectProgramId);
    }

    if root_account.data_len() < 32 || verifier_account.data_len() < 32 {
        return Err(ProgramError::InvalidAccountData);
    }

    let mut offset = 0;
    let public_len = data[offset] as usize;
    offset += 1;
    let public_end = offset + public_len;
    if public_end > data.len() {
        return Err(ProgramError::InvalidInstructionData);
    }
    let public_inputs = &data[offset..public_end];
    offset = public_end;

    if offset >= data.len() {
        return Err(ProgramError::InvalidInstructionData);
    }
    let proof_len = data[offset] as usize;
    offset += 1;
    let proof_end = offset + proof_len;
    if proof_end > data.len() {
        return Err(ProgramError::InvalidInstructionData);
    }
    let proof_blob = &data[offset..proof_end];

    let stored_root = {
        let data_ref = root_account.data.borrow();
        &data_ref[..32]
    };

    let digest = hashv(&[stored_root, public_inputs, proof_blob]);

    let verifier_commitment = {
        let vk = verifier_account.data.borrow();
        &vk[..32]
    };

    if digest.0 != verifier_commitment {
        return Err(ProgramError::InvalidInstructionData);
    }

    msg!("Zero-knowledge proof verified against commitment");
    Ok(())
}

fn compute_merkle_root(
    leaf: &[u8; 32],
    path: &[[u8; 32]],
    directions: &[u8],
) -> [u8; 32] {
    let mut current = *leaf;
    for (idx, sibling) in path.iter().enumerate() {
        let dir = directions[idx];
        current = if dir == 0 {
            hash_pair(&current, sibling)
        } else {
            hash_pair(sibling, &current)
        };
    }
    current
}

fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let digest = hashv(&[left, right]);
    digest.0
}
