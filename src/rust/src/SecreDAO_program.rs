use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, msg, program::invoke,
    program_error::ProgramError, program_option::COption, program_pack::Pack, pubkey::Pubkey,
};

use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account_idempotent,
};
use spl_token::state::{Account, Mint};

entrypoint!(process);

pub fn process(_: &Pubkey, accs: &[AccountInfo], data: &[u8]) -> ProgramResult {
    match data.first() {
        Some(0) => mint_and_init_ata(accs, data),
        Some(1) => burn_and_close_ata(accs, data),
        Some(2) => burn_with_encrypted_receipt(accs, data),
        Some(3) => transfer_and_close_sender(accs, data),
        Some(4) => batch_reward_mint(accs, data),
        _ => Err(ProgramError::InvalidInstructionData),
    }
}


fn mint_and_init_ata(accs: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (signer, mint, ata, owner, token_program, ata_program, system_program, rent) = (
        &accs[0], &accs[1], &accs[2], &accs[3], &accs[4], &accs[5], &accs[6], &accs[7],
    );
    if !signer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if *token_program.key != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    if *mint.owner != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    let mint_data = Mint::unpack(&mint.data.borrow())?;
    if mint_data.mint_authority != COption::Some(*signer.key) {
        return Err(ProgramError::IllegalOwner);
    }
    let expected_ata = get_associated_token_address(owner.key, mint.key);
    if expected_ata != *ata.key {
        return Err(ProgramError::InvalidAccountData);
    }

    if data.len() < 9 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let amount = u64::from_le_bytes(
        data[1..9]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );
    let decimals = mint_data.decimals;

    invoke(
        &create_associated_token_account_idempotent(
            signer.key,
            owner.key,
            mint.key,
            token_program.key,
        ),
        &[
            signer.clone(),
            ata.clone(),
            owner.clone(),
            mint.clone(),
            system_program.clone(),
            token_program.clone(),
            rent.clone(),
            ata_program.clone(),
        ],
    )?;

    invoke(
        &spl_token::instruction::mint_to_checked(
            token_program.key,
            mint.key,
            ata.key,
            signer.key,
            &[],
            amount,
            decimals,
        )?,
        &[
            mint.clone(),
            ata.clone(),
            signer.clone(),
            token_program.clone(),
        ],
    )?;
    Ok(())
}



fn burn_and_close_ata(accs: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let (signer, ata, mint, token_program) = (&accs[0], &accs[1], &accs[2], &accs[3]);
    if data.len() < 9 {
        return Err(ProgramError::InvalidInstructionData);
    }

    if !signer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if *token_program.key != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *mint.owner != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let token_data = spl_token::state::Account::unpack(&ata.try_borrow_data()?)?;

    if token_data.owner != *signer.key {
        return Err(ProgramError::IllegalOwner);
    }

    if token_data.mint != *mint.key {
        return Err(ProgramError::InvalidAccountData);
    }

    let expected_ata = get_associated_token_address(signer.key, mint.key);
    if *ata.key != expected_ata {
        return Err(ProgramError::InvalidAccountData);
    }

    let token_data = spl_token::state::Account::unpack(&ata.try_borrow_data()?)?;

    if token_data.owner != *signer.key {
        return Err(ProgramError::IllegalOwner);
    }

    if token_data.mint != *mint.key {
        return Err(ProgramError::InvalidAccountData);
    }

    if token_data.delegate.is_some() {
        return Err(ProgramError::InvalidAccountData);
    }

    if token_data.close_authority.is_some() {
        return Err(ProgramError::InvalidAccountData);
    }

    let amount = u64::from_le_bytes(
        data[1..9]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );
    invoke(
        &spl_token::instruction::burn(
            token_program.key,
            ata.key,
            mint.key,
            signer.key,
            &[],
            amount,
        )?,
        &[
            ata.clone(),
            mint.clone(),
            signer.clone(),
            token_program.clone(),
        ],
    )?;
    let token_data = spl_token::state::Account::unpack(&ata.try_borrow_data()?)?;
    if token_data.amount == 0 {
        invoke(
            &spl_token::instruction::close_account(
                token_program.key,
                ata.key,
                signer.key,
                signer.key,
                &[],
            )?,
            &[
                ata.clone(),
                signer.clone(),
                signer.clone(),
                token_program.clone(),
            ],
        )?;
    }
    Ok(())
}



fn burn_with_encrypted_receipt(accs: &[AccountInfo], data: &[u8]) -> ProgramResult {

    let (signer, ata, mint, redeem, token_program) =
        (&accs[0], &accs[1], &accs[2], &accs[3], &accs[4]);

    if data.len() < 10 {
        return Err(ProgramError::InvalidInstructionData);
    }

    if !signer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if *token_program.key != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *mint.owner != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *redeem.key != *signer.key {
        return Err(ProgramError::InvalidAccountData);
    }

    let token_data = Account::unpack(&ata.try_borrow_data()?)?;

    if token_data.owner != *signer.key {
        return Err(ProgramError::IllegalOwner);
    }

    if token_data.mint != *mint.key {
        return Err(ProgramError::InvalidAccountData);
    }

    let expected_ata = get_associated_token_address(signer.key, mint.key);
    if *ata.key != expected_ata {
        return Err(ProgramError::InvalidAccountData);
    }

    if token_data.delegate.is_some() {
        return Err(ProgramError::InvalidAccountData);
    }

    if token_data.close_authority.is_some() {
        return Err(ProgramError::InvalidAccountData);
    }
    let amount = u64::from_le_bytes(
        data[1..9]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );
    let has_receiver = data[9];

    match has_receiver {
        0 => {}
        1 => {
            if data.len() < 38 {
                return Err(ProgramError::InvalidInstructionData);
            }

            let nonce = &data[10..22];
            let cipher = &data[22..data.len() - 16];
            let tag = &data[data.len() - 16..];

            msg!(
                "receiver encrypted payload detected: nonce={} bytes, cipher={} bytes, tag={} bytes",
                nonce.len(),
                cipher.len(),
                tag.len()
            );
        }
        _ => return Err(ProgramError::InvalidInstructionData),
    }

    invoke(
        &spl_token::instruction::burn(
            token_program.key,
            ata.key,
            mint.key,
            signer.key,
            &[],
            amount,
        )?,
        &[
            ata.clone(),
            mint.clone(),
            signer.clone(),
            token_program.clone(),
        ],
    )?;

    let token_data = Account::unpack(&ata.try_borrow_data()?)?;
    if token_data.amount == 0 {
        invoke(
            &spl_token::instruction::close_account(
                token_program.key,
                ata.key,
                redeem.key,
                signer.key,
                &[],
            )?,
            &[
                ata.clone(),
                redeem.clone(),
                signer.clone(),
                token_program.clone(),
            ],
        )?;

        msg!("ATA closed");
    }

    Ok(())
}


fn transfer_and_close_sender(accs: &[AccountInfo], data: &[u8]) -> ProgramResult {


    let (signer, from_ata, to_ata, owner_from, mint, token_program) =
        (&accs[0], &accs[1], &accs[2], &accs[3], &accs[4], &accs[5]);

    if data.len() < 9 {
        return Err(ProgramError::InvalidInstructionData);
    }

    if !signer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if *token_program.key != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    if *mint.owner != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let from_data = Account::unpack(&from_ata.try_borrow_data()?)?;

    if from_data.owner != *signer.key {
        return Err(ProgramError::IllegalOwner);
    }

    if from_data.mint != *mint.key {
        return Err(ProgramError::InvalidAccountData);
    }

    if from_data.delegate.is_some() {
        return Err(ProgramError::InvalidAccountData);
    }

    if from_data.close_authority.is_some() {
        return Err(ProgramError::InvalidAccountData);
    }

    let expected_from_ata =
        get_associated_token_address(owner_from.key, mint.key);

    if *from_ata.key != expected_from_ata {
        return Err(ProgramError::InvalidAccountData);
    }

    let amount = u64::from_le_bytes(
        data[1..9]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );
    invoke(
        &spl_token::instruction::transfer(
            token_program.key,
            from_ata.key,
            to_ata.key,
            signer.key,
            &[],
            amount,
        )?,
        &[
            from_ata.clone(),
            to_ata.clone(),
            signer.clone(),
            token_program.clone(),
        ],
    )?;

    let from_data = Account::unpack(&from_ata.try_borrow_data()?)?;

    if from_data.amount == 0 {
        invoke(
            &spl_token::instruction::close_account(
                token_program.key,
                from_ata.key,
                signer.key,
                signer.key,
                &[],
            )?,
            &[
                from_ata.clone(),
                signer.clone(),
                signer.clone(),
                token_program.clone(),
            ],
        )?;
    }

    Ok(())
}
fn batch_reward_mint(accs: &[AccountInfo], data: &[u8]) -> ProgramResult {
    let signer = &accs[0];
    let mint = &accs[1];
    let token_program = &accs[2];
    let ata_program = &accs[3];
    let system_program = &accs[4];
    let rent = &accs[5];

    if !signer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if *token_program.key != spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let mint_data = Mint::unpack(&mint.data.borrow())?;

    if mint_data.mint_authority != COption::Some(*signer.key) {
        return Err(ProgramError::IllegalOwner);
    }

    let decimals = mint_data.decimals;

    let amount = u64::from_le_bytes(
        data[1..9]
            .try_into()
            .map_err(|_| ProgramError::InvalidInstructionData)?,
    );

    let mut i = 6;

    while i < accs.len() {

        let owner = &accs[i];
        let ata = &accs[i + 1];

        let expected_ata =
            get_associated_token_address(owner.key, mint.key);

        if expected_ata != *ata.key {
            return Err(ProgramError::InvalidAccountData);
        }

        invoke(
            &create_associated_token_account_idempotent(
                signer.key,
                owner.key,
                mint.key,
                token_program.key,
            ),
            &[
                signer.clone(),
                ata.clone(),
                owner.clone(),
                mint.clone(),
                system_program.clone(),
                token_program.clone(),
                rent.clone(),
                ata_program.clone(),
            ],
        )?;

        invoke(
            &spl_token::instruction::mint_to_checked(
                token_program.key,
                mint.key,
                ata.key,
                signer.key,
                &[],
                amount,
                decimals,
            )?,
            &[
                mint.clone(),
                ata.clone(),
                signer.clone(),
                token_program.clone(),
            ],
        )?;

        i += 2;
    }
    Ok(())
}