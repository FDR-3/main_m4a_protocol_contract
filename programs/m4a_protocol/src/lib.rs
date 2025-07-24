use anchor_lang::prelude::*;
use anchor_lang::solana_program::system_program;
use anchor_spl::token::{self, Token, TokenAccount};
use core::mem::size_of;
use solana_security_txt::security_txt;

declare_id!("7NNuzG9sACEcwT6bL3TxmnScBbvDARZZjE4tdGZoz5Gm");

#[cfg(not(feature = "no-entrypoint"))] // Ensure it's not included when compiled as a library
security_txt! {
    name: "M4A Protocol",
    project_url: "https://m4a.io",
    contacts: "email fdr3@m4a.io",
    preferred_languages: "en",
    source_code: "https://github.com/FDR-3?tab=repositories",
    policy: "If you find a bug, email me and say something please D:"
}

const SYSTEM_PROGRAM_ADDRESS: Pubkey = pubkey!("11111111111111111111111111111111");
const INITIAL_CEO_ADDRESS: Pubkey = pubkey!("Fdqu1muWocA5ms8VmTrUxRxxmSattrmpNraQ7RpPvzZg");

// Define the constant public key for the USDC fee recipient
pub const INITIAL_TREASURER_ADDRESS: Pubkey = pubkey!("9BRgCdmwyP5wGVTvKAUDjSwucpqGncurVa35DjaWqSsC");

const FEE_4CENTS: f64 = 0.04;

//Patients need atleast 57 extra bytes of space to pass with full load
const PATIENT_EXTRA_SIZE: usize = 64;

//Claims need atleast 288 extra bytes of space to pass with full load
const CLAIM_EXTRA_SIZE: usize = 300;

//Hospitals need atleast 254 extra bytes of space to pass with full load
const HOSPITAL_EXTRA_SIZE: usize = 264;

//Insurance companies need atleast 138 extra bytes of space to pass with full load
const INSURANCE_COMPANY_EXTRA_SIZE: usize = 144;

//Patient records need atleast 143 extra bytes of space to pass with full load
const PATIENT_RECORD_EXTRA_SIZE: usize = 150;

//Hospital records need atleast 139 extra bytes of space to pass with full load
const HOSPITAL_RECORD_EXTRA_SIZE: usize = 144;

//Insurance company records need atleast 141 extra bytes of space to pass with full load
const INSURANCE_COMPANY_RECORD_EXTRA_SIZE: usize = 144;

//Processed claims need atleast 284 extra bytes of space to pass with full load
const PROCESSED_CLAIM_EXTRA_SIZE: usize = 290;

const MAX_NOTE_LENGTH: usize = 144;
const MAX_PATIENT_FIRST_NAME_LENGTH: usize = 52;
const MAX_PATIENT_LAST_NAME_LENGTH: usize = 52;
const MAX_HOSPITAL_NAME_LENGTH: usize = 50;
const MAX_HOSPITAL_ADDRESS_LENGTH: usize = 100;
const MAX_HOSPITAL_CITY_LENGTH: usize = 40;
const MAX_HOSPITAL_BILL_INVOICE_NUMBER_LENGTH: usize = 20;
const MAX_AILMENT_LENGTH: usize = 45;
const MAX_INSURANCE_COMPANY_NAME_LENGTH: usize = 35;

enum Status
{
    Pending = 0,
    Processing = 1,
    Approved = 2,
    Denied = 3,
    Appealed = 4
}

enum HospitalType
{
    General = 0,
    Dental = 1,
    Vision = 2,
    Mental = 3
}

//Error Codes
#[error_code]
pub enum AuthorizationError 
{
    #[msg("Only the CEO can call this function")]
    NotCEO,
    #[msg("Only the Treasurer can call this function")]
    NotTreasurer,
    #[msg("Only a Super Admin or the CEO can call this function")]
    NotSuperAdminOrCEO,
    #[msg("Only an active processor can call this function")]
    NotActiveProcessor,
    #[msg("Only the person who is processing the claim can call this function")]
    NotTheProcessor,
    #[msg("Only the submitter can call this function")]
    NotSubmitter,
    #[msg("A claim can only have one processor")]
    ClaimAlreadyHasProcessor,
    #[msg("A processor can only assign themselves to one claim at a time")]
    ProcessorAlreadyWorkingOnClaim
}  

#[error_code]
pub enum InvalidOperationError
{
    #[msg("You're doing something the UI never would have allowed")]
    NoRatFuckeryAllowed,
    #[msg("The Claim Queue is full")]
    TooManyClaimsInQueue,
    #[msg("Claim Queue is currently disabled")]
    ClaimQueueDisabled,
    #[msg("Can't set flag to the same state")]
    FlagSameState,
    #[msg("Record has already been created")]
    RecordAlreadyCreated,
    #[msg("Record hasn't been created yet")]
    RecordNotCreated,
    #[msg("Claim must not be assigned to assign it")]
    ClaimAlreadyAssigned,
    #[msg("Claim must be assigned to unassign or reassign it")]
    ClaimNotAssigned,
    #[msg("Claim must be being in a pending state to use this Max Deny")]
    ClaimNotPending,
    #[msg("Claim must be being processed already to need be reassigned, denied, or Max inprogress denied")]
    ClaimNotBeingProcessed,
    #[msg("Claim must be in a denied state to appeal it")]
    ClaimNotDenied,
    #[msg("Can't deny appeal of a claim that isn't in an appealed state")]
    ClaimNotAppealed,
    #[msg("Claim must be in a denied or appealed state to undeny it")]
    ClaimNotDeniedOrAppealed,
    #[msg("Claim must be in a approved state to revoke approval")]
    ClaimNotApproved
}   

#[error_code]
pub enum InvalidLengthError 
{
    #[msg("Patient First Name can't be longer than 52 characters")]
    PatientFirstNameTooLong,
    #[msg("Patient Last Name can't be longer than 52 characters")]
    PatientLastNameTooLong,
    #[msg("Hospital Name can't be longer than 100 characters")]
    HospitalNameTooLong,
    #[msg("Hospital Address can't be longer than 100 characters")]
    HospitalAddressTooLong,
    #[msg("Hospital City can't be longer than 40 characters")]
    HospitalCityTooLong,
    #[msg("Hospital Phone Number can't be longer than 20 characters")]
    HospitalPhoneNumberTooLong,
    #[msg("Hospital Bill Invoice Number can't be longer than 20 characters")]
    HospitalBillInvoiceNumberTooLong,
    #[msg("Ailment can't be longer than 45 characters")]
    AilmentTooLong,
    #[msg("Note can't be longer than 140 characters")]
    NoteTooLong,
    #[msg("Insurance company name can't be longer than 35 characters")]
    InsuranceCompanyNameTooLong
}  

#[error_code]
pub enum InvalidType 
{
    #[msg("Hospital type must be General, Dental, Vision, or Mental (0,1,2,3)")]
    HospitalTypeInvalid
}

// Helper function to handle the USDC fee transfer
fn apply_fee<'info>(
    from_account: AccountInfo<'info>,
    to_account: AccountInfo<'info>,
    signer: AccountInfo<'info>,
    token_program: AccountInfo<'info>,
    treasurer: Account<M4AProtocolTreasurer>,
    amount: f64,
    decimal_amount: u8
) -> Result<()> {
    let cpi_accounts = token::Transfer {
        from: from_account,
        to: to_account.clone(),
        authority: signer,
    };
    let cpi_program = token_program;
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

    let base_int :u64 = 10;
    let conversion_number = base_int.pow(decimal_amount as u32) as f64;
    let fixed_pointed_notation_amount = (amount * conversion_number) as u64;

    //Transfer fee to Treasurer Wallet
    token::transfer(cpi_ctx, fixed_pointed_notation_amount)?;
    
    msg!("Successfully transferred ${:.2} as fee to: {}", amount, treasurer.address);

    Ok(())
}

//Functions
#[program]
pub mod m_4_a_protocol 
{
    use super::*;

    pub fn initialize_m4a_protocol_admin_accounts(ctx: Context<InitializeAdminAccounts>) -> Result<()> 
    {
        //Only the initial CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), INITIAL_CEO_ADDRESS, AuthorizationError::NotCEO);

        let ceo = &mut ctx.accounts.ceo;
        ceo.address = INITIAL_CEO_ADDRESS;

        let treasurer = &mut ctx.accounts.treasurer;
        treasurer.address = INITIAL_TREASURER_ADDRESS;

        msg!("M4A Protocol Admin Accounts Initialized");
        msg!("New CEO Address: {}", ceo.address.key());
        msg!("New Treasurer Address: {}", treasurer.address.key());

        Ok(())
    }

    pub fn pass_on_m4a_protocol_ceo(ctx: Context<PassOnM4AProtocolCEO>, new_ceo_address: Pubkey) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        msg!("The M4A Protocol CEO has passed on the title to a new CEO");
        msg!("New CEO: {}", new_ceo_address.key());

        ceo.address = new_ceo_address.key();

        Ok(())
    }

    pub fn pass_on_m4a_protocol_treasurer(ctx: Context<PassOnM4AProtocolTreasurer>, new_treasurer_address: Pubkey) -> Result<()> 
    {
        let treasurer = &mut ctx.accounts.treasurer;
        //Only the Treasurer can call this function
        require_keys_eq!(ctx.accounts.signer.key(), treasurer.address.key(), AuthorizationError::NotTreasurer);

        msg!("The M4A Protocol Treasurer has passed on the title to a new Treasurer");
        msg!("New Treasurer: {}", new_treasurer_address.key());

        treasurer.address = new_treasurer_address.key();

        Ok(())
    }

    pub fn add_fee_token_entry(ctx: Context<AddFeeTokenEntry>, token_mint_address: Pubkey, decimal_amount: u8) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        let fee_token_entry = &mut ctx.accounts.fee_token_entry;
        fee_token_entry.token_mint_address = token_mint_address;
        fee_token_entry.decimal_amount = decimal_amount;

        msg!("Added Fee Token Entry");
        msg!("Mint Address: {}", token_mint_address.key());
        msg!("Decimal Amount: {}", decimal_amount);
            
        Ok(())
    }

    pub fn remove_fee_token_entry(ctx: Context<RemoveFeeTokenEntry>,
        token_mint_address: Pubkey) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        msg!("Removed Fee Token Entry");
        msg!("Mint Address: {}", token_mint_address.key());
            
        Ok(())
    }

    pub fn initialize_m4a_protocol_and_claim_queue(ctx: Context<InitializeM4AProtocolAndClaimQueue>) -> Result<()> 
    {
        let m4a_protocol = &mut ctx.accounts.m4a_protocol;
        m4a_protocol.m4a_protocol_initiator_address = ctx.accounts.signer.key();

        let claim_queue = &mut ctx.accounts.claim_queue;
        claim_queue.enabled = true;
        claim_queue.queue_size_limit = 100;//Set Claim Queue initial size to 100

        msg!("M4A Protocol And Claim Que Initialized");
        msg!("Initialized By User: {}", ctx.accounts.signer.key());

        Ok(())
    }

    pub fn initialize_protocol_stats(ctx: Context<InitializeProtocolStats>) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        msg!("Protocol Stats Initialized");
        msg!("Initialized By User: {}", ctx.accounts.signer.key());

        Ok(())
    }

    pub fn set_claim_queue_flag(ctx: Context<SetClaimQueueFlag>, is_enabled: bool) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        let claim_queue = &mut ctx.accounts.claim_queue;
        claim_queue.enabled = is_enabled;
        
        msg!("Set Claim Queue Flag");
        msg!("Set to {}", is_enabled);
        
        Ok(())
    }

    pub fn edit_claim_queue_size(ctx: Context<EditClaimQueueSize>, new_size_limit: u32) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        let claim_queue = &mut ctx.accounts.claim_queue;
        claim_queue.queue_size_limit = new_size_limit;

        msg!("Claim Queue Initialized");
        Ok(())
    }

    pub fn create_submitter_account(ctx: Context<CreateSubmitterAccount>) -> Result<()> 
    {
        let m4a_protocol = &mut ctx.accounts.m4a_protocol;
        m4a_protocol.submitter_account_total += 1;

        let submitter = &mut ctx.accounts.submitter;
        submitter.id = m4a_protocol.submitter_account_total;
        submitter.address = ctx.accounts.signer.key();

        msg!("Sumitter Account Initialized");
        msg!("User Address: {}", ctx.accounts.signer.key());

        Ok(())
    }

    pub fn create_patient_account(ctx: Context<CreatePatientAccount>, patient_first_name: String, patient_last_name: String) -> Result<()> 
    {
        //Patient first name string must not be longer than 52 characters
        require!(patient_first_name.len() <= MAX_PATIENT_FIRST_NAME_LENGTH, InvalidLengthError::PatientFirstNameTooLong);

        //Patient last name string must not be longer than 52 characters
        require!(patient_last_name.len() <= MAX_PATIENT_LAST_NAME_LENGTH, InvalidLengthError::PatientLastNameTooLong);

        let m4a_protocol = &mut ctx.accounts.m4a_protocol;
        let submitter = &mut ctx.accounts.submitter;
        let patient = &mut ctx.accounts.patient;

        patient.is_active = true;
        patient.submitter_address = ctx.accounts.signer.key();
        patient.patient_first_name = patient_first_name.clone();
        patient.patient_last_name = patient_last_name.clone();

        m4a_protocol.patient_account_total += 1;
        patient.id = m4a_protocol.patient_account_total;
        submitter.active_patient_count += 1;
        
        msg!("Patient Account Initialized");
        msg!("Submitter Address: {}", ctx.accounts.signer.key());
        msg!("Patient Index: {}", submitter.patient_count);
        msg!("Patient First Name: {}", patient_first_name);
        msg!("Patient Last Name: {}", patient_last_name);

        submitter.patient_count += 1;
        
        Ok(())
    }

    pub fn set_patient_flag(ctx: Context<SetPatientFlag>, _patient_index: u8, is_enabled: bool) -> Result<()> 
    {
        let patient = &mut ctx.accounts.patient;
        //Can't set patient to the same state because of the counter
        require!(patient.is_active != is_enabled, InvalidOperationError::FlagSameState);

        let submitter = &mut ctx.accounts.submitter;
        
        patient.is_active = is_enabled;

        if is_enabled
        {
            submitter.active_patient_count += 1; 
        }
        else
        {
            submitter.active_patient_count -= 1; 
        }
        
        msg!("Patient Flag Updated To: {}", is_enabled);
        msg!("Patient First Name: {}", patient.patient_first_name);
        msg!("Patient Last Name: {}", patient.patient_last_name);
        msg!("Submitter Address: {}", ctx.accounts.signer.key());
        
        Ok(())
    }
    
    pub fn create_processor_account(ctx: Context<CreateProcessorAccount>, processor_address: Pubkey) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        let processor_stats = &mut ctx.accounts.processor_stats;
        processor_stats.processor_account_total += 1;
        processor_stats.processor_active_account_total += 1;

        let processor = &mut ctx.accounts.processor;
        processor.id = processor_stats.processor_account_total;
        processor.address = processor_address.key();
        processor.is_active = true;

        msg!("Processor Account Initialized");
        msg!("Processor Address: {}", processor_address.key());
        msg!("Processor Account Count: {}", processor_stats.processor_account_total);
        
        Ok(())
    }

    pub fn set_processor_account_active_flag(ctx: Context<SetProcessorAccountActiveFlag>, processor_address: Pubkey, is_active: bool) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        let processor = &mut ctx.accounts.processor;
        //The flag can't be set to the same state to keep the counters safe
        require!(processor.is_active != is_active, InvalidOperationError::FlagSameState);

        let processor_stats = &mut ctx.accounts.processor_stats;
        processor_stats.edited_processor_count += 1;
        processor.is_active = is_active;

        if is_active == false
        {
            processor_stats.processor_active_account_total -= 1;

            if processor.is_super_admin == true
            {
                processor.is_super_admin = false;
                processor_stats.processor_super_admin_account_total -= 1;
            }
        }
        else
        {
            processor_stats.processor_active_account_total += 1;
        }
        
        msg!("Processor Account Is Active Flag Set To: {}", is_active);
        msg!("Processor Address: {}", processor_address.key());

        Ok(())
    }

    pub fn set_processor_account_privilege(ctx: Context<SetProcessorAccountPrivilege>, processor_address: Pubkey, is_super_admin: bool) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        let processor = &mut ctx.accounts.processor;
        //The flag can't be set to the same state to keep the counters safe
        require!(processor.is_super_admin != is_super_admin, InvalidOperationError::FlagSameState);

        let processor_stats = &mut ctx.accounts.processor_stats;
        processor_stats.edited_processor_count += 1;
        processor.is_super_admin = is_super_admin;

        if is_super_admin == false
        {
            processor_stats.processor_super_admin_account_total -= 1;
        }
        else
        {
            processor_stats.processor_super_admin_account_total += 1;

            if processor.is_active == false
            {
                processor.is_active = true;
                processor_stats.processor_active_account_total += 1;
            }
        }

        msg!("Processor Account Admin Flag Set To: {}", is_super_admin);
        msg!("Processor Address: {}", processor_address.key());

        Ok(())
    }

    pub fn submit_claim_to_queue(ctx: Context<SubmitClaimToQueue>,
        patient_index: u8,
        _token_mint_address: Pubkey,
        country_index: u16,
        state_index: u32,
        hospital_index: i32,
        hospital_type: u8,
        hospital_name: String,
        hospital_address: String,
        hospital_city: String,
        hospital_zip_code: u32,
        hospital_phone_number: u128,
        hospital_bill_invoice_number: String,
        note: String,
        claim_amount: u64,
        ailment: String,
        insurance_company_index: i16,
        insurance_company_name: String
    ) -> Result<()> 
    {
        let claim = &mut ctx.accounts.claim;
        let claim_queue = &mut ctx.accounts.claim_queue;

        //Claim Queue is currently disabled
        require!(claim_queue.enabled == true, InvalidOperationError::ClaimQueueDisabled);

        //You can only submit 1 claim at a time
        //require!(claim.is_active == false, InvalidOperationError::TooManyActiveClaims);

        //Claim Queue is full
        require!(claim_queue.current_claim_queue_count + 1 <= claim_queue.queue_size_limit, InvalidOperationError::TooManyClaimsInQueue);

        //Hospital type must be valid
        require!((hospital_type == HospitalType::General as u8) ||
        (hospital_type == HospitalType::Dental as u8) ||
        (hospital_type == HospitalType::Vision as u8) ||
        (hospital_type == HospitalType::Mental as u8), InvalidType::HospitalTypeInvalid);

        //Hospital name string must not be longer than 50 characters
        require!(hospital_name.len() <= MAX_HOSPITAL_NAME_LENGTH, InvalidLengthError::HospitalNameTooLong);

        //Hospital address string must not be longer than 100 characters
        require!(hospital_address.len() <= MAX_HOSPITAL_ADDRESS_LENGTH, InvalidLengthError::HospitalAddressTooLong);

        //Hospital city string must not be longer than 40 characters
        require!(hospital_city.len() <= MAX_HOSPITAL_CITY_LENGTH, InvalidLengthError::HospitalCityTooLong);

        //Hospital bill invoice number string must not be longer than 20 characters
        require!(hospital_bill_invoice_number.len() <= MAX_HOSPITAL_BILL_INVOICE_NUMBER_LENGTH, InvalidLengthError::HospitalBillInvoiceNumberTooLong);

        //Ailment string must not be longer than 45 characters
        require!(ailment.len() <= MAX_AILMENT_LENGTH, InvalidLengthError::AilmentTooLong);

        //Note string must not be longer than 140 characters
        require!(note.len() <= MAX_NOTE_LENGTH, InvalidLengthError::NoteTooLong);

        //Insurance company name string must not be longer than 35 characters
        require!(insurance_company_name.len() <= MAX_INSURANCE_COMPANY_NAME_LENGTH, InvalidLengthError::InsuranceCompanyNameTooLong);

        let submitter = &mut ctx.accounts.submitter;
        let patient = &mut ctx.accounts.patient;

        claim_queue.submitted_claim_count += 1;
        claim_queue.current_claim_queue_count += 1;
        patient.submitted_claim_count += 1;
        submitter.submitted_claim_count += 1;
        
        claim.id = claim_queue.submitted_claim_count;
        claim.submitter_address = ctx.accounts.signer.key();
        claim.patient_index = patient_index;
        claim.country_index = country_index.clone();
        claim.state_index = state_index.clone();
        claim.hospital_index = hospital_index;
        claim.hospital_type = hospital_type;
        claim.hospital_name = hospital_name;
        claim.hospital_address = hospital_address;
        claim.hospital_city = hospital_city;
        claim.hospital_zip_code = hospital_zip_code;
        claim.hospital_phone_number = hospital_phone_number;
        claim.hospital_bill_invoice_number = hospital_bill_invoice_number;
        claim.note = note;
        claim.claim_amount = claim_amount.clone();
        claim.ailment = ailment.clone();
        claim.insurance_company_index = insurance_company_index;
        claim.insurance_company_name = insurance_company_name;
        claim.submitted_time = Clock::get()?.unix_timestamp as u64;
        
        msg!("New Claim Submited to the Queue");
        msg!("Submitter Address: {}", ctx.accounts.signer.key());
        msg!("Patient First Name: {}", patient.patient_first_name);
        msg!("Patient Last Name: {}", patient.patient_last_name);
        msg!("Country Index: {}", country_index);
        msg!("State Index: {}", state_index);
        msg!("Hospital Index: {}", hospital_index);
        msg!("Hospital Type: {}", hospital_type);
        msg!("Claim Info: {}", ailment);
        msg!("For: ${:.2}", claim_amount as f64/100.00);
        msg!("Note: {}", claim.note);

        let accounts = &ctx.accounts;
        let treasurer = ctx.accounts.treasurer.clone();

        //Call the helper function to transfer the fee
        apply_fee(
            accounts.user_fee_ata.to_account_info(),
            accounts.treasurer_usdc_ata.to_account_info(),
            accounts.signer.to_account_info(),
            accounts.token_program.to_account_info(),
            treasurer,
            FEE_4CENTS,
            accounts.fee_token_entry.decimal_amount
        )?;

        Ok(())
    }

    pub fn assign_claim_to_processor(ctx: Context<AssignClaimToProcessor>, submitter_address: Pubkey) -> Result<()> 
    {
        let processor_stats = &mut ctx.accounts.processor_stats;
        let processor = &mut ctx.accounts.processor;
        let claim = &mut ctx.accounts.claim;
        
        //Only an active Processor can call this function
        require!(processor.is_active == true, AuthorizationError::NotActiveProcessor);

        //Processor must not already be processing any other claim
        require!(processor.is_processing_claim == false, AuthorizationError::ProcessorAlreadyWorkingOnClaim);

        //A claim can only have one processor
        require_keys_eq!(claim.processor_address.key(), SYSTEM_PROGRAM_ADDRESS.key(), InvalidOperationError::ClaimAlreadyAssigned);

        processor.is_processing_claim = true;
        processor.submitter_address_of_claim_being_processed = submitter_address.key();
        claim.processor_address = ctx.accounts.signer.key();
        claim.status = Status::Processing as u8;
        processor_stats.set_or_unset_processor_on_claim_count += 1;

        msg!("Claim Assigned To Processor Address: ");
        msg!("{}", ctx.accounts.signer.key());

        Ok(())
    }

    pub fn reassign_claim_to_new_processor(ctx: Context<ReassignClaimToNewProcessor>, submitter_address: Pubkey) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        let processor_stats = &mut ctx.accounts.processor_stats;
        let new_processor = &mut ctx.accounts.new_processor;
        let old_processor = &mut ctx.accounts.old_processor;
        let claim = &mut ctx.accounts.claim;

        //Only an Admin or the CEO can call this function
        require!(ctx.accounts.signer.key() == ceo.address.key() ||
        new_processor.is_super_admin == true, AuthorizationError::NotSuperAdminOrCEO);

        //Processor must not already be processing any other claim
        require!(new_processor.is_processing_claim == false, AuthorizationError::ProcessorAlreadyWorkingOnClaim);

        //A claim can not be unassigned or reassigned if it isn't currently assigned
        require_keys_neq!(claim.processor_address.key(), SYSTEM_PROGRAM_ADDRESS.key(), InvalidOperationError::ClaimNotAssigned);

        new_processor.is_processing_claim = true;
        new_processor.submitter_address_of_claim_being_processed = submitter_address.key();
        processor_stats.set_or_unset_processor_on_claim_count += 1;

        //Check if processor is reassigning themself to the same claim for some weird ass reason, do nothing else if so
        if new_processor.address != claim.processor_address
        {
            old_processor.is_processing_claim = false;
            old_processor.submitter_address_of_claim_being_processed = SYSTEM_PROGRAM_ADDRESS;
        }

        msg!("Claim Reassigned To New Processor Address: ");
        msg!("{}", ctx.accounts.signer.key());
        msg!("Old Processor Address: ");
        msg!("{}", claim.processor_address);

        claim.processor_address = ctx.accounts.signer.key();

        Ok(())
    }

    pub fn unassign_claim_from_processor(ctx: Context<UnassignClaimFromProcessor>, _submitter_address: Pubkey) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        let processor_stats = &mut ctx.accounts.processor_stats;
        let admin_processor = &mut ctx.accounts.admin_processor;
        let old_processor = &mut ctx.accounts.old_processor;
        let claim = &mut ctx.accounts.claim;

        //Only an Admin or the CEO can call this function
        require!(ctx.accounts.signer.key() == ceo.address.key() ||
        admin_processor.is_super_admin == true, AuthorizationError::NotSuperAdminOrCEO);

        //A claim can not be unassigned or reassigned if it isn't currently assigned
        require_keys_neq!(claim.processor_address.key(), SYSTEM_PROGRAM_ADDRESS.key(), InvalidOperationError::ClaimNotAssigned);

        old_processor.is_processing_claim = false;
        old_processor.submitter_address_of_claim_being_processed = SYSTEM_PROGRAM_ADDRESS;
        claim.processor_address = SYSTEM_PROGRAM_ADDRESS;
        claim.status = Status::Pending as u8;

        processor_stats.set_or_unset_processor_on_claim_count += 1;

        msg!("Claim id: {} Unassigned By: ", claim.id);
        msg!("{}", ctx.accounts.signer.key());

        Ok(())
    }

    //For in the event that the claim has already been denied some kind of way and the processor is stuck on a dead claim (Denial Hammer most likely)
    pub fn set_processor_to_not_processing_claim_state(ctx: Context<SetProcessorToNotProcessingClaimState>, _processor_address: Pubkey) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        let processor_stats = &mut ctx.accounts.processor_stats;
        let admin_processor = &mut ctx.accounts.admin_processor;
        let processor = &mut ctx.accounts.processor;

        //Only an Admin or the CEO can call this function
        require!(ctx.accounts.signer.key() == ceo.address.key() ||
        admin_processor.is_super_admin == true, AuthorizationError::NotSuperAdminOrCEO);

        processor.is_processing_claim = false;
        processor.submitter_address_of_claim_being_processed = SYSTEM_PROGRAM_ADDRESS;
        processor_stats.set_or_unset_processor_on_claim_count += 1;

        msg!("Processor Set To Not Processign Claim State By: ");
        msg!("{}", ctx.accounts.signer.key());

        Ok(())
    }

    pub fn create_state_account(ctx: Context<CreateStateAccount>, _submitter_address: Pubkey, country_index: u16, state_index: u32) -> Result<()> 
    {
        let claim = &mut ctx.accounts.claim;
        let processor = &mut ctx.accounts.processor;
        
        //Only an active Processor can call this function
        require!(processor.is_active == true, AuthorizationError::NotActiveProcessor);

        //Only the Processor can call this function
        require_keys_eq!(processor.submitter_address_of_claim_being_processed.key(), claim.submitter_address.key(), AuthorizationError::NotTheProcessor);
        
        let m4a_protocol = &mut ctx.accounts.m4a_protocol;
        let state = &mut ctx.accounts.state;

        m4a_protocol.state_account_total += 1;
        state.id = m4a_protocol.state_account_total;
        state.index = state_index;
        
        msg!("Initialized Country at Index: {}", country_index);
        msg!("Initialized State at Index: {}", state_index);
        msg!("State Id: {}", state_index);

        Ok(())
    }

    pub fn create_hospital(ctx: Context<CreateHospital>, 
        _submitter_address: Pubkey,
        country_index: u16,
        state_index: u32,
        hospital_type: u8,
        hospital_longitude: f64,
        hospital_latitude: f64,
        hospital_name: String,
        hospital_address: String,
        hospital_city: String,
        hospital_zip_code: u32,
        hospital_phone_number: u128,
        note: String) -> Result<()> 
    { 
        let claim = &mut ctx.accounts.claim;
        let processor = &mut ctx.accounts.processor;
        
        //Only an active Processor can call this function
        require!(processor.is_active == true, AuthorizationError::NotActiveProcessor);

        //Only the Processor can call this function
        require_keys_eq!(processor.submitter_address_of_claim_being_processed.key(), claim.submitter_address.key(), AuthorizationError::NotTheProcessor);

        //Hospital type must be valid
        require!((hospital_type == HospitalType::General as u8) ||
        (hospital_type == HospitalType::Dental as u8) ||
        (hospital_type == HospitalType::Vision as u8) ||
        (hospital_type == HospitalType::Mental as u8), InvalidType::HospitalTypeInvalid);

        //Hospital name string must not be longer than 50 characters
        require!(hospital_name.len() <= MAX_HOSPITAL_NAME_LENGTH, InvalidLengthError::HospitalNameTooLong);

        //Hospital address string must not be longer than 100 characters
        require!(hospital_address.len() <= MAX_HOSPITAL_ADDRESS_LENGTH, InvalidLengthError::HospitalAddressTooLong);

        //Hospital city string must not be longer than 40 characters
        require!(hospital_city.len() <= MAX_HOSPITAL_CITY_LENGTH, InvalidLengthError::HospitalCityTooLong);

        //Note string must not be longer than 140 characters
        require!(note.len() <= MAX_NOTE_LENGTH, InvalidLengthError::NoteTooLong);
        
        let hospital_stats = &mut ctx.accounts.hospital_stats;
        let processor = &mut ctx.accounts.processor;
        let state = &mut ctx.accounts.state;
        let hospital = &mut ctx.accounts.hospital;
        
        hospital_stats.hospital_count += 1;
        processor.created_hospital_count += 1;

        claim.country_index = country_index;
        claim.state_index = state_index;
        claim.hospital_type = hospital_type;
        claim.hospital_index = state.hospital_count as i32;
        claim.hospital_name = hospital_name.clone();
        claim.hospital_address = hospital_address.clone();
        claim.hospital_city = hospital_city.clone();
        claim.hospital_zip_code = hospital_zip_code;
        claim.hospital_phone_number = hospital_phone_number.clone();

        hospital.id = hospital_stats.hospital_count;
        hospital.hospital_index = state.hospital_count;
        hospital.is_active = true;
        hospital.country_index = country_index;
        hospital.state_index = state_index;
        hospital.hospital_type = hospital_type;
        hospital.hospital_longitude = hospital_longitude;
        hospital.hospital_latitude = hospital_latitude;
        hospital.hospital_name = hospital_name;
        hospital.hospital_address = hospital_address;
        hospital.hospital_city = hospital_city;
        hospital.hospital_zip_code = hospital_zip_code;
        hospital.hospital_phone_number = hospital_phone_number;
        hospital.note = note;

        state.hospital_count += 1;

        if hospital_type == HospitalType::General as u8
        {
            hospital_stats.general_hospital_count += 1;
            state.general_hospital_count += 1;
        }
        else if hospital_type == HospitalType::Dental as u8
        {
            hospital_stats.dental_hospital_count += 1;
            state.dental_hospital_count += 1;
        }
        else if hospital_type == HospitalType::Vision as u8
        {
            hospital_stats.vision_hospital_count += 1;
            state.vision_hospital_count += 1;
        }
        else if hospital_type == HospitalType::Mental as u8
        {
            hospital_stats.mental_hospital_count += 1;
            state.mental_hospital_count += 1;
        }

        msg!("Hospital Created #{}", hospital.id);
        msg!("Country Index: {}", country_index);
        msg!("State Index: {}", state_index);
        msg!("Hospital Index: {}", state.hospital_count-1);
        msg!("Hospital Type: {}", hospital.hospital_type);
        msg!("Longitude: {}", hospital_longitude);
        msg!("Latitude: {}", hospital_latitude);
        msg!("Note: {}", hospital.note.clone());
        msg!("State Hospital Count: {}", state.hospital_count);
        msg!("M4A Protocol General Hospital Count: {}", hospital_stats.hospital_count);

        Ok(())
    }

    pub fn edit_hospital(ctx: Context<EditHospital>, 
        country_index: u16,
        state_index: u32,
        hospital_index: u32,
        is_active: bool,
        hospital_type: u8,
        hospital_longitude: f64,
        hospital_latitude: f64,
        hospital_name: String,
        hospital_address: String,
        hospital_city: String,
        hospital_zip_code: u32,
        hospital_phone_number: u128,
        note: String) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        //Hospital type must be valid
        require!((hospital_type == HospitalType::General as u8) ||
        (hospital_type == HospitalType::Dental as u8) ||
        (hospital_type == HospitalType::Vision as u8) ||
        (hospital_type == HospitalType::Mental as u8), InvalidType::HospitalTypeInvalid);

        //Hospital name string must not be longer than 50 characters
        require!(hospital_name.len() <= MAX_HOSPITAL_NAME_LENGTH, InvalidLengthError::HospitalNameTooLong);

        //Hospital address string must not be longer than 100 characters
        require!(hospital_address.len() <= MAX_HOSPITAL_ADDRESS_LENGTH, InvalidLengthError::HospitalNameTooLong);

        //Hospital city string must not be longer than 40 characters
        require!(hospital_city.len() <= MAX_HOSPITAL_CITY_LENGTH, InvalidLengthError::HospitalNameTooLong);

        //Note string must not be longer than 140 characters
        require!(note.len() <= MAX_NOTE_LENGTH, InvalidLengthError::HospitalNameTooLong);

        let hospital_stats = &mut ctx.accounts.hospital_stats;
        let state = &mut ctx.accounts.state;
        let hospital = &mut ctx.accounts.hospital;

        //Wait to deduct previous hospital type before setting it to the hospital
        hospital.is_active = is_active;
        hospital.hospital_longitude = hospital_longitude;
        hospital.hospital_latitude = hospital_latitude;
        hospital.hospital_name = hospital_name;
        hospital.hospital_address = hospital_address;
        hospital.hospital_city = hospital_city;
        hospital.hospital_zip_code = hospital_zip_code;
        hospital.hospital_phone_number = hospital_phone_number;
        hospital.note = note;

        //Deduct previous type from count
        if hospital.hospital_type == HospitalType::General as u8
        {
            hospital_stats.general_hospital_count -= 1;
            state.general_hospital_count -= 1;
        }
        else if hospital.hospital_type == HospitalType::Dental as u8
        {
            hospital_stats.dental_hospital_count -= 1;
            state.dental_hospital_count -= 1;
        }
        else if hospital.hospital_type == HospitalType::Vision as u8
        {
            hospital_stats.vision_hospital_count -= 1;
            state.vision_hospital_count -= 1;
        }
        else if hospital.hospital_type == HospitalType::Mental as u8
        {
            hospital_stats.mental_hospital_count -= 1;
            state.mental_hospital_count -= 1;
        }

        //Add new type to count
        if hospital_type == HospitalType::General as u8
        {
            hospital_stats.general_hospital_count += 1;
            state.general_hospital_count += 1;
        }
        else if hospital_type == HospitalType::Dental as u8
        {
            hospital_stats.dental_hospital_count += 1;
            state.dental_hospital_count += 1;
        }
        else if hospital_type == HospitalType::Vision as u8
        {
            hospital_stats.vision_hospital_count += 1;
            state.vision_hospital_count += 1;
        }
        else if hospital_type == HospitalType::Mental as u8
        {
            hospital_stats.mental_hospital_count += 1;
            state.mental_hospital_count += 1;
        }

        hospital_stats.edited_hospital_count += 1;
        state.edited_hospital_count += 1;
    
        //Set new hospital type
        hospital.hospital_type = hospital_type;

        msg!("Hospital Edited");
        msg!("Country Index: {}", country_index);
        msg!("State Index: {}", state_index);
        msg!("Hospital Index: {}", hospital_index);
        msg!("Hospital Active: {}", is_active);
        msg!("Hospital Type: {}", hospital.hospital_type);
        msg!("Hospital Name: {}", hospital.hospital_name);
        msg!("Hospital Address: {}", hospital.hospital_address);
        msg!("Hospital City: {}", hospital.hospital_city);
        msg!("Hospital ZipCode: {}", hospital.hospital_zip_code);
        msg!("Hospital PhoneNumber: {}", hospital.hospital_phone_number);
        msg!("Longitude: {}", hospital_longitude);
        msg!("Latitude: {}", hospital_latitude);
        msg!("Note: {}", hospital.note.clone());

        Ok(())
    }

    pub fn create_insurance_company(ctx: Context<CreateInsuranceCompany>, 
        _submitter_address: Pubkey, 
        insurance_company_index: u16,
        insurance_company_name: String,
        note: String) -> Result<()> 
    {
        let claim = &mut ctx.accounts.claim;
        let processor = &mut ctx.accounts.processor;
        
        //Only an active Processor can call this function
        require!(processor.is_active == true, AuthorizationError::NotActiveProcessor);

        //Only the Processor can call this function
        require_keys_eq!(processor.submitter_address_of_claim_being_processed.key(), claim.submitter_address.key(), AuthorizationError::NotTheProcessor);

        //Insurance company name string must not be longer than 35 characters
        require!(insurance_company_name.len() <= MAX_INSURANCE_COMPANY_NAME_LENGTH, InvalidLengthError::InsuranceCompanyNameTooLong);

        //Note string must not be longer than 140 characters
        require!(note.len() <= MAX_NOTE_LENGTH, InvalidLengthError::NoteTooLong);

        let insurance_company_stats = &mut ctx.accounts.insurance_company_stats;
        let processor = &mut ctx.accounts.processor;
        let insurance_company = &mut ctx.accounts.insurance_company;
        
        claim.insurance_company_index = insurance_company_index as i16;
        claim.insurance_company_name = insurance_company_name.clone();
   
        insurance_company.is_active = true;
        insurance_company.note = note;
        insurance_company.insurance_company_name = insurance_company_name.clone();
        
        insurance_company_stats.initialized_insurance_company_count += 1;
        insurance_company.id = insurance_company_stats.initialized_insurance_company_count;
        insurance_company.insurance_company_index = insurance_company_index;

        if insurance_company_index > 10
        {
            insurance_company_stats.additional_insurance_company_count += 1;
        }

        processor.created_insurance_company_count += 1;

        msg!("Insurance Company Initialized");
        msg!("Insurance Company Index: {}", insurance_company_index);
        msg!("Insurance Company Name: {}", insurance_company_name.clone());
        msg!("Note: {}", insurance_company.note.clone());

        Ok(())
    }

    pub fn edit_insurance_company(ctx: Context<EditInsuranceCompany>, 
        insurance_company_index: u16,
        is_active: bool,
        insurance_company_name: String,
        note: String) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        //Insurance company name string must not be longer than 35 characters
        require!(insurance_company_name.len() <= MAX_INSURANCE_COMPANY_NAME_LENGTH, InvalidLengthError::InsuranceCompanyNameTooLong);

        //Note string must not be longer than 140 characters
        require!(note.len() <= MAX_NOTE_LENGTH, InvalidLengthError::NoteTooLong);

        let insurance_company_stats = &mut ctx.accounts.insurance_company_stats;
        let insurance_company = &mut ctx.accounts.insurance_company;
        
        insurance_company.is_active = is_active;
        insurance_company.insurance_company_name = insurance_company_name.clone();
        insurance_company.note = note.clone();

        insurance_company_stats.edited_insurance_company_count += 1;

        msg!("Insurance Company Edited");
        msg!("Insurance Company Index: {}", insurance_company_index);
        msg!("Insurance Company Active: {}", is_active);
        msg!("Insurance Company Name: {}", insurance_company_name);
        msg!("Note: {}", note);

        Ok(())
    }

    pub fn update_claim_hospital_index(ctx: Context<UpdateClaim>,
        _submitter_address: Pubkey,
        hospital_index: u32
    ) -> Result<()> 
    {
        let claim = &mut ctx.accounts.claim;
        let processor = &mut ctx.accounts.processor;
        
        //Only an active Processor can call this function
        require!(processor.is_active == true, AuthorizationError::NotActiveProcessor);

        //Only the Processor can call this function
        require_keys_eq!(processor.submitter_address_of_claim_being_processed.key(), claim.submitter_address.key(), AuthorizationError::NotTheProcessor);

        //Can't set different hospital index after hospital record has been created
        require!(claim.is_hospital_record_created == false, InvalidOperationError::RecordAlreadyCreated);
 
        let processor_stats = &mut ctx.accounts.processor_stats;

        processor_stats.edited_claim_or_processed_claim_count += 1;
        claim.hospital_index = hospital_index as i32;
        
        msg!("Claim Hospital Index updated");
        msg!("Hospital Index: {}", hospital_index);

        Ok(())
    }

    pub fn update_claim_insurance_company_index(ctx: Context<UpdateClaim>,
        _submitter_address: Pubkey,
        insurance_company_index: u16
    ) -> Result<()> 
    {
        let claim = &mut ctx.accounts.claim;
        let processor = &mut ctx.accounts.processor;
        
        //Only an active Processor can call this function
        require!(processor.is_active == true, AuthorizationError::NotActiveProcessor);

        //Only the Processor can call this function
        require_keys_eq!(processor.submitter_address_of_claim_being_processed.key(), claim.submitter_address.key(), AuthorizationError::NotTheProcessor);

        //Can't set different insurance company index after insurance company record has been created
        require!(claim.is_insurance_company_record_created == false, InvalidOperationError::RecordAlreadyCreated);

        let processor_stats = &mut ctx.accounts.processor_stats;

        processor_stats.edited_claim_or_processed_claim_count += 1;
        claim.insurance_company_index = insurance_company_index as i16;
        
        msg!("Claim Insurance Company Index updated");
        msg!("Insurance Company Index: {}", insurance_company_index);

        Ok(())
    }

    pub fn create_patient_record(ctx: Context<CreatePatientRecord>, _submitter_address: Pubkey) -> Result<()> 
    {
        let claim = &mut ctx.accounts.claim;
        let processor = &mut ctx.accounts.processor;
        
        //Only an active Processor can call this function
        require!(processor.is_active == true, AuthorizationError::NotActiveProcessor);

        //Only the Processor can call this function
        require_keys_eq!(processor.submitter_address_of_claim_being_processed.key(), claim.submitter_address.key(), AuthorizationError::NotTheProcessor);

        //Only create 1 patient record per claim
        require!(claim.is_patient_record_created == false, InvalidOperationError::RecordAlreadyCreated);

        let processor_stats = &mut ctx.accounts.processor_stats;
        processor_stats.created_patient_record_count += 1;
        
        let patient = &mut ctx.accounts.patient;
        let patient_record = &mut ctx.accounts.patient_record;

        claim.patient_record_index = patient.record_count;
        claim.is_patient_record_created = true;
        patient.record_count += 1;
        patient_record.record_id = patient.record_count as u32;
        patient_record.claim_id = claim.id as u32;
        patient_record.status = Status::Processing as u8;
        patient_record.patient_record_only = true;
        patient_record.submitter_address = claim.submitter_address;
        patient_record.processor_address = ctx.accounts.signer.key();
        patient_record.country_index = claim.country_index;
        patient_record.state_index = claim.state_index;
        patient_record.hospital_index = claim.hospital_index as u32;
        patient_record.hospital_bill_invoice_number = claim.hospital_bill_invoice_number.clone();
        patient_record.claim_amount = claim.claim_amount;
        patient_record.ailment = claim.ailment.clone();
        patient_record.note = claim.note.clone();
        patient_record.submitted_time = claim.submitted_time;
        patient_record.insurance_company_index = claim.insurance_company_index as u16;

        processor.created_patient_record_count += 1;

        msg!("Patient Record Created");
        msg!("Record ID: {}", patient.record_count);
        msg!("Claim ID: {}", patient_record.claim_id);
        msg!("Submitter Address: {}", claim.submitter_address);
        msg!("Patient Index: {}", claim.patient_index);
        msg!("Claim Amount: ${:.2}", patient_record.claim_amount as f64/100.00);
        
        Ok(())
    }

    pub fn create_hospital_and_insurance_company_records(ctx: Context<CreateHospitalAndInsuranceCompanyRecords>, _submitter_address: Pubkey) -> Result<()> 
    {
        let claim = &mut ctx.accounts.claim;
        let processor = &mut ctx.accounts.processor;
        
        //Only an active Processor can call this function
        require!(processor.is_active == true, AuthorizationError::NotActiveProcessor);

        //Only the Processor can call this function
        require_keys_eq!(processor.submitter_address_of_claim_being_processed.key(), claim.submitter_address.key(), AuthorizationError::NotTheProcessor);

        //Patient Record must already exist
        require!(claim.is_patient_record_created == true, InvalidOperationError::RecordNotCreated);

        //Only create 1 hospital record per claim
        require!(claim.is_hospital_record_created == false, InvalidOperationError::RecordAlreadyCreated);

        //Only create 1 insurance company record per claim
        require!(claim.is_insurance_company_record_created == false, InvalidOperationError::RecordAlreadyCreated);

        let processor_stats = &mut ctx.accounts.processor_stats;
        processor_stats.created_hospital_and_insurance_company_records_count += 1;

        let patient_record = &mut ctx.accounts.patient_record;
        patient_record.patient_record_only = false;

        let hospital = &mut ctx.accounts.hospital;
        let hospital_record = &mut ctx.accounts.hospital_record;

        claim.hospital_record_index = hospital.record_count;
        claim.is_hospital_record_created = true;
        hospital.record_count += 1;
        hospital_record.record_id = hospital.record_count;
        hospital_record.claim_id = claim.id;
        hospital_record.status = Status::Processing as u8;
        hospital_record.submitter_address = claim.submitter_address;
        hospital_record.patient_index = claim.patient_index;
        hospital_record.processor_address = ctx.accounts.signer.key();
        hospital_record.claim_amount = claim.claim_amount;
        hospital_record.hospital_bill_invoice_number = claim.hospital_bill_invoice_number.clone();
        hospital_record.ailment = claim.ailment.clone();
        hospital_record.note = claim.note.clone();
        hospital_record.submitted_time = claim.submitted_time;
        hospital_record.insurance_company_index = claim.insurance_company_index  as u16;
        
        processor.created_hospital_record_count += 1;

        msg!("Hospital Record Created");
        msg!("Record ID: {}", hospital.record_count);
        msg!("Claim ID: {}", hospital_record.claim_id);
        msg!("Hospital Index: {}", claim.hospital_index);
        msg!("Claim Amount: ${:.2}", hospital_record.claim_amount as f64/100.00);

        let insurance_company = &mut ctx.accounts.insurance_company;
        let insurance_company_record = &mut ctx.accounts.insurance_company_record;

        claim.insurance_company_record_index = insurance_company.record_count;
        claim.is_insurance_company_record_created = true;
        insurance_company.record_count += 1;
        insurance_company_record.record_id = insurance_company.record_count;
        insurance_company_record.claim_id = claim.id;
        insurance_company_record.status = Status::Processing as u8;
        insurance_company_record.submitter_address = claim.submitter_address;
        insurance_company_record.patient_index = claim.patient_index;
        insurance_company_record.processor_address = ctx.accounts.signer.key();
        insurance_company_record.country_index = claim.country_index;
        insurance_company_record.state_index = claim.state_index;
        insurance_company_record.hospital_index = claim.hospital_index as u32;
        insurance_company_record.hospital_bill_invoice_number = claim.hospital_bill_invoice_number.clone();
        insurance_company_record.claim_amount = claim.claim_amount;
        insurance_company_record.ailment = claim.ailment.clone();
        insurance_company_record.note = claim.note.clone();
        insurance_company_record.submitted_time = claim.submitted_time;

        processor.created_insurance_company_record_count += 1;

        msg!("Insurance Company Record Created");
        msg!("Record ID: {}", insurance_company.record_count);
        msg!("Claim ID: {}", insurance_company_record.claim_id);
        msg!("Insurance Company Index: {}", claim.insurance_company_index);
        msg!("Claim Amount: ${:.2}", insurance_company_record.claim_amount as f64/100.00);
        
        Ok(())
    }

    pub fn approve_claim(ctx: Context<ApproveClaim>, _submitter_address: Pubkey) -> Result<()> 
    {
        let claim = &mut ctx.accounts.claim;
        let processor = &mut ctx.accounts.processor;
        
        //Only an active Processor can call this function
        require!(processor.is_active == true, AuthorizationError::NotActiveProcessor);

        //Only the Processor can call this function
        require_keys_eq!(processor.submitter_address_of_claim_being_processed.key(), claim.submitter_address.key(), AuthorizationError::NotTheProcessor);

        let processor_stats = &mut ctx.accounts.processor_stats;
        let claim_queue = &mut ctx.accounts.claim_queue;
        let submitter = &mut ctx.accounts.submitter;
        let patient = &mut ctx.accounts.patient;
        let state = &mut ctx.accounts.state;
        let hospital = &mut ctx.accounts.hospital;
        let insurance_company = &mut ctx.accounts.insurance_company;

        processor_stats.approved_claim_count += 1;
        processor_stats.processed_claim_count += 1;
        processor_stats.approved_claim_amount += claim.claim_amount;
        claim_queue.current_claim_queue_count -= 1;
        submitter.approved_claim_count += 1;
        submitter.approved_claim_amount += claim.claim_amount;
        patient.approved_claim_count += 1;
        patient.approved_claim_amount += claim.claim_amount;
        state.approved_claim_count += 1;
        state.approved_claim_amount += claim.claim_amount;
        hospital.approved_claim_count += 1;
        hospital.approved_claim_amount += claim.claim_amount;
        insurance_company.approved_claim_count += 1;
        insurance_company.approved_claim_amount += claim.claim_amount;
        
        let processed_claim = &mut ctx.accounts.processed_claim;
        processed_claim.processed_claim_id = processor_stats.processed_claim_count;
        processed_claim.claim_id = claim.id;
        processed_claim.processor_count_index = processor.processed_claim_count;
        processed_claim.status = Status::Approved as u8;
        processed_claim.is_patient_record_created = true;
        processed_claim.is_hospital_record_created = true;
        processed_claim.is_insurance_company_record_created = true;
        processed_claim.patient_record_index = claim.patient_record_index;
        processed_claim.hospital_record_index = claim.hospital_record_index;
        processed_claim.insurance_company_record_index = claim.insurance_company_record_index;
        processed_claim.processor_address = ctx.accounts.signer.key();
        processed_claim.submitter_address = claim.submitter_address;
        processed_claim.patient_index = claim.patient_index;
        processed_claim.country_index = claim.country_index;
        processed_claim.state_index = claim.state_index;
        processed_claim.hospital_index = claim.hospital_index;
        processed_claim.hospital_type = claim.hospital_type;
        processed_claim.hospital_name = claim.hospital_name.clone();
        processed_claim.hospital_address = claim.hospital_address.clone();
        processed_claim.hospital_city = claim.hospital_city.clone();
        processed_claim.hospital_zip_code = claim.hospital_zip_code;
        processed_claim.hospital_phone_number = claim.hospital_phone_number.clone();
        processed_claim.hospital_bill_invoice_number = claim.hospital_bill_invoice_number.clone();
        processed_claim.note = claim.note.clone();
        processed_claim.claim_amount = claim.claim_amount;
        processed_claim.ailment = claim.ailment.clone();
        processed_claim.insurance_company_index = claim.insurance_company_index;
        processed_claim.insurance_company_name = claim.insurance_company_name.clone();
        processed_claim.submitted_time = claim.submitted_time;
        processed_claim.processed_time = Clock::get()?.unix_timestamp as u64;

        let patient_record = &mut ctx.accounts.patient_record;
        patient_record.status = Status::Approved as u8;
        patient_record.processor_count_index = processor.processed_claim_count;
        patient_record.processed_time = Clock::get()?.unix_timestamp as u64;

        let hospital_record = &mut ctx.accounts.hospital_record;
        hospital_record.status = Status::Approved as u8;
        hospital_record.processor_count_index = processor.processed_claim_count;
        hospital_record.processed_time = Clock::get()?.unix_timestamp as u64;

        let insurance_company_record = &mut ctx.accounts.insurance_company_record;
        insurance_company_record.status = Status::Approved as u8;
        insurance_company_record.processor_count_index = processor.processed_claim_count;
        insurance_company_record.processed_time = Clock::get()?.unix_timestamp as u64;

        processor.approved_claim_amount += claim.claim_amount;
        processor.approved_claim_count += 1;
        processor.processed_claim_count += 1;
        processor.is_processing_claim = false;

        msg!("New Claim Approved");
        msg!("For: ${:.2}", processed_claim.claim_amount as f64/100.00);
        msg!("Approved Claim Count: {}", processor_stats.approved_claim_count);
        msg!("User Address: {}", processed_claim.submitter_address);
        msg!("Patient First Name: {}", patient.patient_first_name);
        msg!("Patient Last Name: {}", patient.patient_last_name);

        Ok(())
    }

    pub fn approve_claim_with_edits(ctx: Context<ApproveClaimWithEdits>, 
        _submitter_address: Pubkey,
        hospital_type: u8,
        hospital_longitude: f64,
        hospital_latitude: f64,
        hospital_name: String,
        hospital_address: String,
        hospital_city: String,
        hospital_zip_code: u32,
        hospital_phone_number: u128,
        hospital_bill_invoice_number: String,
        claim_note: String,
        claim_amount: u64,
        ailment: String,
        insurance_company_name: String,) -> Result<()> 
    {
        let claim = &mut ctx.accounts.claim;
        let processor = &mut ctx.accounts.processor;
        
        //Only an active Processor can call this function
        require!(processor.is_active == true, AuthorizationError::NotActiveProcessor);

        //Only the Processor can call this function
        require_keys_eq!(processor.submitter_address_of_claim_being_processed.key(), claim.submitter_address.key(), AuthorizationError::NotTheProcessor);

        //Hospital type must be valid
        require!((hospital_type == HospitalType::General as u8) ||
        (hospital_type == HospitalType::Dental as u8) ||
        (hospital_type == HospitalType::Vision as u8) ||
        (hospital_type == HospitalType::Mental as u8), InvalidType::HospitalTypeInvalid);

        //Hospital name string must not be longer than 50 characters
        require!(hospital_name.len() <= MAX_HOSPITAL_NAME_LENGTH, InvalidLengthError::HospitalNameTooLong);

        //Hospital address string must not be longer than 100 characters
        require!(hospital_address.len() <= MAX_HOSPITAL_ADDRESS_LENGTH, InvalidLengthError::HospitalAddressTooLong);

        //Hospital city string must not be longer than 40 characters
        require!(hospital_city.len() <= MAX_HOSPITAL_CITY_LENGTH, InvalidLengthError::HospitalCityTooLong);

        //Hospital bill invoice number string must not be longer than 20 characters
        require!(hospital_bill_invoice_number.len() <= MAX_HOSPITAL_BILL_INVOICE_NUMBER_LENGTH, InvalidLengthError::HospitalBillInvoiceNumberTooLong);

        //Ailment string must not be longer than 45 characters
        require!(ailment.len() <= MAX_AILMENT_LENGTH, InvalidLengthError::AilmentTooLong);

        //Note string must not be longer than 140 characters
        require!(claim_note.len() <= MAX_NOTE_LENGTH, InvalidLengthError::NoteTooLong);

        //Insurance company name string must not be longer than 35 characters
        require!(insurance_company_name.len() <= MAX_INSURANCE_COMPANY_NAME_LENGTH, InvalidLengthError::InsuranceCompanyNameTooLong);

        let processor_stats = &mut ctx.accounts.processor_stats;
        let claim_queue = &mut ctx.accounts.claim_queue;
        let submitter = &mut ctx.accounts.submitter;
        let patient = &mut ctx.accounts.patient;
        let state = &mut ctx.accounts.state;
        let hospital = &mut ctx.accounts.hospital;
        let insurance_company = &mut ctx.accounts.insurance_company;

        //Update Amount Totals & Counts
        processor_stats.approved_claim_count += 1;
        processor_stats.processed_claim_count += 1;
        processor_stats.approved_claim_amount += claim_amount;
        claim_queue.current_claim_queue_count -= 1;
        submitter.approved_claim_count += 1;
        submitter.approved_claim_amount += claim_amount;
        patient.approved_claim_count += 1;
        patient.approved_claim_amount += claim_amount;
        state.approved_claim_count += 1;
        state.approved_claim_amount += claim_amount;
        hospital.approved_claim_count += 1;
        hospital.approved_claim_amount += claim_amount;
        insurance_company.approved_claim_count += 1;
        insurance_company.approved_claim_amount += claim_amount;
        
        //Update Hospital
        hospital.hospital_type = hospital_type;
        hospital.hospital_longitude = hospital_longitude;
        hospital.hospital_latitude = hospital_latitude;
        hospital.hospital_name = hospital_name.clone();
        hospital.hospital_address = hospital_address.clone();
        hospital.hospital_city = hospital_city.clone();
        hospital.hospital_zip_code = hospital_zip_code;
        hospital.hospital_phone_number = hospital_phone_number.clone();

        //Update Insurance Company
        insurance_company.insurance_company_name = insurance_company_name.clone();

        //Update Records
        let patient_record = &mut ctx.accounts.patient_record;
        patient_record.status = Status::Approved as u8;
        patient_record.processor_count_index = processor.processed_claim_count;
        patient_record.hospital_index = claim.hospital_index as u32;
        patient_record.hospital_bill_invoice_number = hospital_bill_invoice_number.clone();
        patient_record.claim_amount = claim_amount;
        patient_record.ailment = ailment.clone();
        patient_record.note = claim_note.clone();
        patient_record.processed_time = Clock::get()?.unix_timestamp as u64;
        patient_record.insurance_company_index = claim.insurance_company_index as u16;

        let hospital_record = &mut ctx.accounts.hospital_record;
        hospital_record.status = Status::Approved as u8;
        hospital_record.processor_count_index = processor.processed_claim_count;
        hospital_record.claim_amount = claim_amount;
        hospital_record.hospital_bill_invoice_number = hospital_bill_invoice_number.clone();
        hospital_record.ailment = ailment.clone();
        hospital_record.note = claim_note.clone();
        hospital_record.processed_time = Clock::get()?.unix_timestamp as u64;
        hospital_record.insurance_company_index = claim.insurance_company_index as u16;

        let insurance_company_record = &mut ctx.accounts.insurance_company_record;
        insurance_company_record.status = Status::Approved as u8;
        insurance_company_record.processor_count_index = processor.processed_claim_count;
        insurance_company_record.hospital_index = claim.hospital_index as u32;
        insurance_company_record.hospital_bill_invoice_number = hospital_bill_invoice_number.clone();
        insurance_company_record.claim_amount = claim_amount;
        insurance_company_record.ailment = ailment.clone();
        insurance_company_record.note = claim_note.clone();
        insurance_company_record.processed_time = Clock::get()?.unix_timestamp as u64;

        //Create Processed Claim
        let processed_claim = &mut ctx.accounts.processed_claim;
        processed_claim.processed_claim_id = processor_stats.processed_claim_count;
        processed_claim.claim_id = claim.id;
        processed_claim.processor_count_index = processor.processed_claim_count;
        processed_claim.status = Status::Approved as u8;
        processed_claim.is_patient_record_created = true;
        processed_claim.is_hospital_record_created = true;
        processed_claim.is_insurance_company_record_created = true;
        processed_claim.patient_record_index = claim.patient_record_index;
        processed_claim.hospital_record_index = claim.hospital_record_index;
        processed_claim.insurance_company_record_index = claim.insurance_company_record_index;
        processed_claim.processor_address = ctx.accounts.signer.key();
        processed_claim.submitter_address = claim.submitter_address;
        processed_claim.patient_index = claim.patient_index;
        processed_claim.country_index = claim.country_index;
        processed_claim.state_index = claim.state_index;
        processed_claim.hospital_index = claim.hospital_index;
        processed_claim.hospital_type = hospital_type;
        processed_claim.hospital_name = hospital_name;
        processed_claim.hospital_address = hospital_address;
        processed_claim.hospital_city = hospital_city;
        processed_claim.hospital_zip_code = hospital_zip_code;
        processed_claim.hospital_phone_number = hospital_phone_number;
        processed_claim.hospital_bill_invoice_number = hospital_bill_invoice_number;
        processed_claim.note = claim_note;
        processed_claim.claim_amount = claim_amount;
        processed_claim.ailment = ailment;
        processed_claim.insurance_company_index = claim.insurance_company_index;
        processed_claim.insurance_company_name = insurance_company_name;
        processed_claim.submitted_time = claim.submitted_time;
        processed_claim.processed_time = Clock::get()?.unix_timestamp as u64;

        processor.approved_claim_amount += claim.claim_amount;
        processor.approved_claim_count += 1;
        processor.processed_claim_count += 1;
        processor.is_processing_claim = false;

        msg!("New Claim Approved With Edits");
        msg!("For: ${:.2}", claim_amount as f64/100.00);
        msg!("Approved Claim Count: {}", processor_stats.approved_claim_count);
        msg!("User Address: {}", claim.submitter_address);
        msg!("Patient First Name: {}", patient.patient_first_name);
        msg!("Patient Last Name: {}", patient.patient_last_name);
        Ok(())
    }

    pub fn max_deny_pending_claim(ctx: Context<MaxDenyPendingClaim>, submitter_address: Pubkey) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        let claim = &mut ctx.accounts.claim;
        let admin_processor = &mut ctx.accounts.admin_processor;

        //Only an Admin or the CEO can call this function
        require!(ctx.accounts.signer.key() == ceo.address.key() ||
        admin_processor.is_super_admin == true, AuthorizationError::NotSuperAdminOrCEO);

        //Claim must be in a pending state to use this Max Deny
        require!(claim.status == Status::Pending as u8, InvalidOperationError::ClaimNotPending);

        //Can't max deny claim if patient record was created
        require!(claim.is_patient_record_created == false, InvalidOperationError::RecordAlreadyCreated);

        //Can't max deny claim if hospital record was created
        require!(claim.is_hospital_record_created == false, InvalidOperationError::RecordAlreadyCreated);

        //Can't max deny claim if insurance company record was created
        require!(claim.is_insurance_company_record_created == false, InvalidOperationError::RecordAlreadyCreated);

        let processor_stats = &mut ctx.accounts.processor_stats;
        let submitter = &mut ctx.accounts.submitter;
        let patient = &mut ctx.accounts.patient;
        processor_stats.max_denied_claim_count += 1;
        submitter.max_denied_claim_count += 1;
        patient.max_denied_claim_count += 1;
        admin_processor.max_denied_claim_count += 1;
     
        let claim_queue = &mut ctx.accounts.claim_queue; 
        claim_queue.current_claim_queue_count -= 1;

        msg!("New Max Pending Claim Denial");
        msg!("Max Denied Claim Count: {}", processor_stats.max_denied_claim_count);
        msg!("User Address: {}", submitter_address);
        
        Ok(())
    }

    pub fn max_deny_in_progress_claim(ctx: Context<MaxDenyInProgressClaim>, submitter_address: Pubkey) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        let claim = &mut ctx.accounts.claim;
        let admin_processor = &mut ctx.accounts.admin_processor;
        let claim_processor = &mut ctx.accounts.claim_processor;

        //Only an Admin or the CEO can call this function
        require!(ctx.accounts.signer.key() == ceo.address.key() ||
        admin_processor.is_super_admin == true, AuthorizationError::NotSuperAdminOrCEO);

        //Claim must be in a processing state to use this Max Deny
        require!(claim.status == Status::Processing as u8, InvalidOperationError::ClaimNotBeingProcessed);

        //Can't max deny claim if patient record was created
        require!(claim.is_patient_record_created == false, InvalidOperationError::RecordAlreadyCreated);

        //Can't max deny claim if hospital record was created
        require!(claim.is_hospital_record_created == false, InvalidOperationError::RecordAlreadyCreated);

        //Can't max deny claim if insurance company record was created
        require!(claim.is_insurance_company_record_created == false, InvalidOperationError::RecordAlreadyCreated);

        let processor_stats = &mut ctx.accounts.processor_stats;
        let submitter = &mut ctx.accounts.submitter;
        let patient = &mut ctx.accounts.patient;
        processor_stats.max_denied_claim_count += 1;
        submitter.max_denied_claim_count += 1;
        patient.max_denied_claim_count += 1;
        admin_processor.max_denied_claim_count += 1;
     
        let claim_queue = &mut ctx.accounts.claim_queue; 
        claim_queue.current_claim_queue_count -= 1;

        if claim.status == Status::Processing as u8
        {
            claim_processor.is_processing_claim = false;

            //Check if Signer was the processor on the claim, they can't exist in 2 processor variables in this function, so have to do an extra check
            if claim.processor_address == ctx.accounts.signer.key()
            {
                admin_processor.is_processing_claim = false;
                claim_processor.max_denied_claim_count += 1;
            }
        }

        msg!("New Max In Progress Claim Denial");
        msg!("Max Denied Claim Count: {}", processor_stats.max_denied_claim_count);
        msg!("User Address: {}", submitter_address);
        
        Ok(())
    }

    pub fn create_patient_record_and_deny_claim(ctx: Context<CreatePatientRecordAndDenyClaim>, _submitter_address: Pubkey, denial_reason: String) -> Result<()> 
    {
        let claim = &mut ctx.accounts.claim;
        let processor = &mut ctx.accounts.processor;
        
        //Only an active Processor can call this function
        require!(processor.is_active == true, AuthorizationError::NotActiveProcessor);

        //Only the Processor can call this function
        require_keys_eq!(processor.submitter_address_of_claim_being_processed.key(), claim.submitter_address.key(), AuthorizationError::NotTheProcessor);

        let state = &mut ctx.accounts.state;
        let processor_stats = &mut ctx.accounts.processor_stats;
        processor_stats.denied_claim_count += 1;
        state.denied_claim_count += 1;
        processor_stats.processed_claim_count += 1;
        processor_stats.created_patient_record_count += 1;

        //Only create 1 patient record per claim
        require!(claim.is_patient_record_created == false, InvalidOperationError::RecordAlreadyCreated);

        //Denial note string must not be longer than 140 characters
        require!(denial_reason.len() <= MAX_NOTE_LENGTH, InvalidLengthError::NoteTooLong);

        let claim_queue = &mut ctx.accounts.claim_queue; 
        claim_queue.current_claim_queue_count -= 1;

        let patient = &mut ctx.accounts.patient;
        let submitter = &mut ctx.accounts.submitter;
        let time_stamp = Clock::get()?.unix_timestamp as u64;

        let processed_claim = &mut ctx.accounts.processed_claim;
        processed_claim.processed_claim_id = processor_stats.processed_claim_count;
        processed_claim.claim_id = claim.id;
        processed_claim.processor_count_index = processor.processed_claim_count;
        processed_claim.status = Status::Denied as u8;
        processed_claim.denial_reason = denial_reason.clone();
        processed_claim.is_patient_record_created = true;
        processed_claim.patient_record_index = patient.record_count;
        processed_claim.processor_address = ctx.accounts.signer.key();
        processed_claim.submitter_address = claim.submitter_address;
        processed_claim.patient_index = claim.patient_index;
        processed_claim.country_index = claim.country_index;
        processed_claim.state_index = claim.state_index;
        processed_claim.hospital_index = claim.hospital_index;
        processed_claim.hospital_name = claim.hospital_name.clone();
        processed_claim.hospital_address = claim.hospital_address.clone();
        processed_claim.hospital_city = claim.hospital_city.clone();
        processed_claim.hospital_zip_code = claim.hospital_zip_code;
        processed_claim.hospital_phone_number = claim.hospital_phone_number.clone();
        processed_claim.hospital_bill_invoice_number = claim.hospital_bill_invoice_number.clone();
        processed_claim.note = claim.note.clone();
        processed_claim.claim_amount = claim.claim_amount;
        processed_claim.ailment = claim.ailment.clone();
        processed_claim.insurance_company_index = claim.insurance_company_index;
        processed_claim.insurance_company_name = claim.insurance_company_name.clone();
        processed_claim.submitted_time = claim.submitted_time;
        processed_claim.processed_time = time_stamp;
        
        let patient_record = &mut ctx.accounts.patient_record;
        patient.record_count += 1;
        patient_record.record_id = patient.record_count as u32;
        patient_record.claim_id = claim.id as u32;
        patient_record.status = Status::Denied as u8;
        patient_record.patient_record_only = true;
        patient_record.submitter_address = claim.submitter_address;
        patient_record.processor_address = ctx.accounts.signer.key();
        patient_record.processor_count_index = processor.processed_claim_count;
        patient_record.denial_reason = denial_reason.clone();
        patient_record.country_index = claim.country_index;
        patient_record.state_index = claim.state_index;
        patient_record.hospital_index = claim.hospital_index as u32;
        patient_record.insurance_company_index = claim.insurance_company_index as u16;
        patient_record.hospital_bill_invoice_number = claim.hospital_bill_invoice_number.clone();
        patient_record.claim_amount = claim.claim_amount;
        patient_record.ailment = claim.ailment.clone();
        patient_record.note = claim.note.clone();
        patient_record.submitted_time = claim.submitted_time;
        patient_record.processed_time = time_stamp;
        
        submitter.denied_claim_count += 1;
        patient.denied_claim_count += 1;

        processor.created_patient_record_count += 1;
        processor.denied_claim_count += 1;
        processor.processed_claim_count += 1;
        processor.is_processing_claim = false;
        
        msg!("New Patient Record And Claim Denial");
        msg!("Denied Claim Count: {}", processor_stats.denied_claim_count);
        msg!("User Address: {}", claim.submitter_address);
        msg!("Reason: {}", denial_reason.clone());

        Ok(())
    }

    pub fn deny_claim_with_all_records(ctx: Context<DenyClaimWithAllRecords>, _submitter_address: Pubkey, denial_reason: String) -> Result<()> 
    {
        let claim = &mut ctx.accounts.claim;
        let processor = &mut ctx.accounts.processor;
        
        //Only an active Processor can call this function
        require!(processor.is_active == true, AuthorizationError::NotActiveProcessor);

        //Only the Processor can call this function
        require_keys_eq!(processor.submitter_address_of_claim_being_processed.key(), claim.submitter_address.key(), AuthorizationError::NotTheProcessor);

        //Only claims being processed can be denied
        require!(claim.status == Status::Processing as u8, InvalidOperationError::ClaimNotBeingProcessed);
        
        //Can't deny claim if patient record wasn't created
        require!(claim.is_patient_record_created == true, InvalidOperationError::RecordAlreadyCreated);

        //Can't deny claim if hospital record wasn't created
        require!(claim.is_hospital_record_created == true, InvalidOperationError::RecordAlreadyCreated);

        //Can't deny claim if insurance company record wasn't created
        require!(claim.is_insurance_company_record_created == true, InvalidOperationError::RecordAlreadyCreated);

        //Denial note string must not be longer than 140 characters
        require!(denial_reason.len() <= MAX_NOTE_LENGTH, InvalidLengthError::NoteTooLong);

        let processor_stats = &mut ctx.accounts.processor_stats;
        let claim_queue = &mut ctx.accounts.claim_queue; 
        let submitter = &mut ctx.accounts.submitter;
        let patient = &mut ctx.accounts.patient;
        let state = &mut ctx.accounts.state;
        let hospital = &mut ctx.accounts.hospital;
        let insurance_company = &mut ctx.accounts.insurance_company;
        let time_stamp = Clock::get()?.unix_timestamp as u64;

        processor_stats.denied_claim_count += 1;
        processor_stats.processed_claim_count += 1;
        claim_queue.current_claim_queue_count -= 1;
        submitter.denied_claim_count += 1;
        patient.denied_claim_count += 1;
        state.denied_claim_count += 1;
        hospital.denied_claim_count += 1;
        insurance_company.denied_claim_count += 1;

        let processed_claim = &mut ctx.accounts.processed_claim;
        processed_claim.processed_claim_id = processor_stats.processed_claim_count;
        processed_claim.claim_id = claim.id;
        processed_claim.processor_count_index = processor.processed_claim_count;
        processed_claim.status = Status::Denied as u8;
        processed_claim.denial_reason = denial_reason.clone();
        processed_claim.is_patient_record_created = true;
        processed_claim.is_hospital_record_created = true;
        processed_claim.is_insurance_company_record_created = true;
        processed_claim.patient_record_index = claim.patient_record_index;
        processed_claim.hospital_record_index = claim.hospital_record_index;
        processed_claim.insurance_company_record_index = claim.insurance_company_record_index;
        processed_claim.processor_address = ctx.accounts.signer.key();
        processed_claim.submitter_address = claim.submitter_address;
        processed_claim.patient_index = claim.patient_index;
        processed_claim.country_index = claim.country_index;
        processed_claim.state_index = claim.state_index;
        processed_claim.hospital_index = claim.hospital_index;
        processed_claim.hospital_name = claim.hospital_name.clone();
        processed_claim.hospital_address = claim.hospital_address.clone();
        processed_claim.hospital_city = claim.hospital_city.clone();
        processed_claim.hospital_zip_code = claim.hospital_zip_code;
        processed_claim.hospital_phone_number = claim.hospital_phone_number.clone();
        processed_claim.hospital_bill_invoice_number = claim.hospital_bill_invoice_number.clone();
        processed_claim.note = claim.note.clone();
        processed_claim.claim_amount = claim.claim_amount;
        processed_claim.ailment = claim.ailment.clone();
        processed_claim.insurance_company_index = claim.insurance_company_index;
        processed_claim.insurance_company_name = claim.insurance_company_name.clone();
        processed_claim.submitted_time = claim.submitted_time;
        processed_claim.processed_time = time_stamp;

        let patient_record = &mut ctx.accounts.patient_record;
        patient_record.status = Status::Denied as u8;
        patient_record.processor_count_index = processor.processed_claim_count;
        patient_record.denial_reason = denial_reason.clone();
        patient_record.processed_time = time_stamp;

        let hospital_record = &mut ctx.accounts.hospital_record;
        hospital_record.status = Status::Denied as u8;
        hospital_record.processor_count_index = processor.processed_claim_count;
        hospital_record.denial_reason = denial_reason.clone();
        hospital_record.processed_time = time_stamp;

        let insurance_company_record = &mut ctx.accounts.insurance_company_record;
        insurance_company_record.status = Status::Denied as u8;
        insurance_company_record.processor_count_index = processor.processed_claim_count;
        insurance_company_record.denial_reason = denial_reason.clone();
        insurance_company_record.processed_time = time_stamp;

        processor.denied_claim_count += 1;
        processor.processed_claim_count += 1;
        processor.is_processing_claim = false;
        
        msg!("New Claim Denial");
        msg!("Denied Claim Count: {}", processor_stats.denied_claim_count);
        msg!("User Address: {}", claim.submitter_address);
        msg!("Reason: {}", denial_reason.clone());
        
        Ok(())
    } 

    pub fn appeal_denied_claim_with_only_patient_record(ctx: Context<AppealDeniedClaimWithOnlyPatientRecord>,
        _processor_address: Pubkey,
        _processor_count_index: u64,
        _token_mint_address: Pubkey,
        appeal_reason: String) -> Result<()> 
    {
        let processed_claim = &mut ctx.accounts.processed_claim;

        //Only the person who submitted the claim can appeal it
        require_keys_eq!(ctx.accounts.signer.key(), processed_claim.submitter_address, AuthorizationError::NotSubmitter);

        //Only denied claims can be appealed
        require!(processed_claim.status == Status::Denied as u8, InvalidOperationError::ClaimNotDenied);
        
        //Prevent Rat Fuckery
        require!(processed_claim.is_patient_record_created == true, InvalidOperationError::NoRatFuckeryAllowed);

        //Prevent Rat Fuckery
        require!(processed_claim.is_hospital_record_created == false, InvalidOperationError::NoRatFuckeryAllowed);

        //Prevent Rat Fuckery
        require!(processed_claim.is_insurance_company_record_created == false, InvalidOperationError::NoRatFuckeryAllowed);

        //Appeal note string must not be longer than 140 characters
        require!(appeal_reason.len() <= MAX_NOTE_LENGTH, InvalidLengthError::NoteTooLong);

        let processor_stats = &mut ctx.accounts.processor_stats;
        let submitter = &mut ctx.accounts.submitter;
        let patient = &mut ctx.accounts.patient;
        let patient_record = &mut ctx.accounts.patient_record;
        let state = &mut ctx.accounts.state;

        processor_stats.submitted_appeal_count += 1;
        submitter.submitted_appeal_count += 1;
        patient.submitted_appeal_count += 1;
        state.submitted_appeal_count += 1;
        patient_record.status = Status::Appealed as u8;
        patient_record.appeal_reason = appeal_reason.clone();
        processed_claim.status = Status::Appealed as u8;
        processed_claim.appeal_reason = appeal_reason.clone();
        
        msg!("New Appeal For Denied Claim With Only Patient Record");
        msg!("Appeal Reason {}", appeal_reason);
        msg!("Submitted Appeals Count {}", processor_stats.submitted_appeal_count);

        let accounts = &ctx.accounts;
        let treasurer = ctx.accounts.treasurer.clone();

        //Call the helper function to transfer the fee
        apply_fee(
            accounts.user_fee_ata.to_account_info(),
            accounts.treasurer_usdc_ata.to_account_info(),
            accounts.signer.to_account_info(),
            accounts.token_program.to_account_info(),
            treasurer,
            FEE_4CENTS,
            accounts.fee_token_entry.decimal_amount
        )?;

        Ok(())
    }

    pub fn deny_appealed_claim_with_only_patient_record(ctx: Context<DenyAppealedClaimWithOnlyPatientRecord>, _processor_address: Pubkey, _processor_count_index: u64, denial_reason: String) -> Result<()> 
    {
        let processed_claim = &mut ctx.accounts.processed_claim;

        //Can't deny appeal of a claim that isn't in an appealed state
        require!(processed_claim.status == Status::Appealed as u8, InvalidOperationError::ClaimNotAppealed);

        //Can't deny appeal of a claim that isn't in an appealed state
        require!(processed_claim.status == Status::Appealed as u8, InvalidOperationError::ClaimNotAppealed);

        //Prevent Rat Fuckery
        require!(processed_claim.is_patient_record_created == true, InvalidOperationError::NoRatFuckeryAllowed);

        //Prevent Rat Fuckery
        require!(processed_claim.is_hospital_record_created == false, InvalidOperationError::NoRatFuckeryAllowed);

        //Prevent Rat Fuckery
        require!(processed_claim.is_insurance_company_record_created == false, InvalidOperationError::NoRatFuckeryAllowed);

        //Denital note string must not be longer than 140 characters
        require!(denial_reason.len() <= MAX_NOTE_LENGTH, InvalidLengthError::NoteTooLong);

        let processor_stats = &mut ctx.accounts.processor_stats;
        let submitter = &mut ctx.accounts.submitter;
        let patient = &mut ctx.accounts.patient;
        let processor = &mut ctx.accounts.processor;
        let state = &mut ctx.accounts.state;
        let patient_record = &mut ctx.accounts.patient_record;
        
        let time_stamp = Clock::get()?.unix_timestamp as u64;

        processor_stats.denied_appeal_count += 1;
        submitter.denied_appeal_count += 1;
        patient.denied_appeal_count += 1;
        processor.denied_appeal_count += 1;
        state.denied_appeal_count += 1;
        patient_record.status = Status::Denied as u8;
        patient_record.denial_reason = denial_reason.clone();
        patient_record.processed_time = time_stamp;
        processed_claim.status = Status::Denied as u8;
        processed_claim.denial_reason = denial_reason.clone();
        processed_claim.processed_time = time_stamp;
        
        msg!("An Appeal With Only A Patient Record Has Been Denied");
        msg!("Denital Reason {}", denial_reason);
        msg!("Submitted Appeals Count {}", processor_stats.denied_appeal_count);

        Ok(())
    }

    pub fn appeal_denied_claim_with_all_records(ctx: Context<AppealDeniedClaimWithAllRecords>,
        _processor_address: Pubkey,
        _processor_count_index: u64,
        _token_mint_address: Pubkey,
        appeal_reason: String) -> Result<()> 
    {
        let processed_claim = &mut ctx.accounts.processed_claim;

        //Only the person who submitted the claim can appeal it
        require_keys_eq!(ctx.accounts.signer.key(), processed_claim.submitter_address, AuthorizationError::NotSubmitter);

        //Only denied claims can be appealed
        require!(processed_claim.status == Status::Denied as u8, InvalidOperationError::ClaimNotDenied);

        //Prevent Rat Fuckery
        require!(processed_claim.is_patient_record_created == true, InvalidOperationError::NoRatFuckeryAllowed);

        //Prevent Rat Fuckery
        require!(processed_claim.is_hospital_record_created == true, InvalidOperationError::NoRatFuckeryAllowed);

        //Prevent Rat Fuckery
        require!(processed_claim.is_insurance_company_record_created == true, InvalidOperationError::NoRatFuckeryAllowed);

        //Appeal note string must not be longer than 140 characters
        require!(appeal_reason.len() <= MAX_NOTE_LENGTH, InvalidLengthError::NoteTooLong);

        let processor_stats = &mut ctx.accounts.processor_stats;
        let state = &mut ctx.accounts.state;
        let patient = &mut ctx.accounts.patient;
        let patient_record = &mut ctx.accounts.patient_record;
        let hospital = &mut ctx.accounts.hospital;
        let hospital_record = &mut ctx.accounts.hospital_record;
        let insurance_company = &mut ctx.accounts.insurance_company;
        let insurance_company_record = &mut ctx.accounts.insurance_company_record;
        
        processor_stats.submitted_appeal_count += 1;
        state.submitted_appeal_count += 1;
        processed_claim.status = Status::Appealed as u8;
        processed_claim.appeal_reason = appeal_reason.clone();
        patient.submitted_appeal_count += 1;
        patient_record.status = Status::Appealed as u8;
        patient_record.appeal_reason = appeal_reason.clone();
        hospital.submitted_appeal_count += 1;
        hospital_record.status = Status::Appealed as u8;
        hospital_record.appeal_reason = appeal_reason.clone();
        insurance_company.submitted_appeal_count += 1;
        insurance_company_record.status = Status::Appealed as u8;
        insurance_company_record.appeal_reason = appeal_reason.clone();
        
        msg!("New Appeal For Denied Claim With All Records");
        msg!("Appeal Reason {}", appeal_reason);
        msg!("Submitted Appeals Count {}", processor_stats.submitted_appeal_count);

        let accounts = &ctx.accounts;
        let treasurer = ctx.accounts.treasurer.clone();

        //Call the helper function to transfer the fee
        apply_fee(
            accounts.user_fee_ata.to_account_info(),
            accounts.treasurer_usdc_ata.to_account_info(),
            accounts.signer.to_account_info(),
            accounts.token_program.to_account_info(),
            treasurer,
            FEE_4CENTS,
            accounts.fee_token_entry.decimal_amount
        )?;

        Ok(())
    }

    pub fn deny_appealed_claim_with_all_records(ctx: Context<DenyAppealedClaimWithAllRecords>, _processor_address: Pubkey, _processor_count_index: u64, denial_reason: String) -> Result<()> 
    {
        let processed_claim = &mut ctx.accounts.processed_claim;

        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        //Can't deny appeal of a claim that isn't in an appealed state
        require!(processed_claim.status == Status::Appealed as u8, InvalidOperationError::ClaimNotAppealed);

        //Prevent Rat Fuckery
        require!(processed_claim.is_patient_record_created == true, InvalidOperationError::NoRatFuckeryAllowed);

        //Prevent Rat Fuckery
        require!(processed_claim.is_hospital_record_created == true, InvalidOperationError::NoRatFuckeryAllowed);

        //Prevent Rat Fuckery
        require!(processed_claim.is_insurance_company_record_created == true, InvalidOperationError::NoRatFuckeryAllowed);

        //Denial note string must not be longer than 140 characters
        require!(denial_reason.len() <= MAX_NOTE_LENGTH, InvalidLengthError::NoteTooLong);

        let processor_stats = &mut ctx.accounts.processor_stats;
        let submitter = &mut ctx.accounts.submitter;
        let patient = &mut ctx.accounts.patient;
        let processor = &mut ctx.accounts.processor;
        let state = &mut ctx.accounts.state;
        let patient_record = &mut ctx.accounts.patient_record;
        let hospital = &mut ctx.accounts.hospital;
        let hospital_record = &mut ctx.accounts.hospital_record;
        let insurance_company = &mut ctx.accounts.insurance_company;
        let insurance_company_record = &mut ctx.accounts.insurance_company_record;
        let time_stamp = Clock::get()?.unix_timestamp as u64;
        
        processor_stats.denied_appeal_count += 1;
        processor.denied_appeal_count += 1;
        submitter.denied_appeal_count += 1;
        patient.denied_appeal_count += 1;
        processor.denied_appeal_count += 1;
        state.denied_appeal_count += 1;
        patient_record.status = Status::Denied as u8;
        patient_record.denial_reason = denial_reason.clone();
        patient_record.processed_time = time_stamp;
        hospital.denied_appeal_count += 1;
        hospital_record.status = Status::Denied as u8;
        hospital_record.denial_reason = denial_reason.clone();
        hospital_record.processed_time = time_stamp;
        insurance_company.denied_appeal_count += 1;
        insurance_company_record.status = Status::Denied as u8;
        insurance_company_record.denial_reason = denial_reason.clone();
        insurance_company_record.processed_time = time_stamp;
        processed_claim.status = Status::Denied as u8;
        processed_claim.denial_reason = denial_reason.clone();
        processed_claim.processed_time = time_stamp;
        
        msg!("An Appeal With Only All Records Has Been Denied");
        msg!("Denital Reason {}", denial_reason);
        msg!("Submitted Appeals Count {}", processor_stats.denied_appeal_count);

        Ok(())
    }

    pub fn undeny_claim_and_create_hospital_and_insurance_company_records(ctx: Context<UndenyClaimAndCreateHospitalAndInsuranceCompanyRecords>, _processor_address: Pubkey, _processor_count_index: u64) -> Result<()> 
    {
        let processed_claim = &mut ctx.accounts.processed_claim;

        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        //Only denied or appealed claims can be undenied
        require!((processed_claim.status == Status::Denied as u8) || (processed_claim.status == Status::Appealed as u8), InvalidOperationError::ClaimNotDeniedOrAppealed);

        let processor_stats = &mut ctx.accounts.processor_stats;
        let submitter = &mut ctx.accounts.submitter;
        let patient = &mut ctx.accounts.patient;
        let processor = &mut ctx.accounts.processor;
        let state = &mut ctx.accounts.state;
        let hospital = &mut ctx.accounts.hospital;
        let insurance_company = &mut ctx.accounts.insurance_company;
        let time_stamp = Clock::get()?.unix_timestamp as u64;

        processor_stats.approved_claim_amount += processed_claim.claim_amount;
        processor_stats.undenied_claim_count += 1;
        processor_stats.approved_claim_count += 1;
        processor_stats.denied_claim_count -= 1;
        processor_stats.created_hospital_and_insurance_company_records_count += 1;
        submitter.undenied_claim_count += 1;
        submitter.approved_claim_count += 1;
        submitter.denied_claim_count -= 1;
        submitter.approved_claim_amount += processed_claim.claim_amount;
        patient.undenied_claim_count += 1;
        patient.approved_claim_count += 1;
        patient.denied_claim_count -= 1;
        patient.approved_claim_amount += processed_claim.claim_amount;
        processor.undenied_claim_count += 1;
        processor.approved_claim_amount += processed_claim.claim_amount;
        state.undenied_claim_count += 1;
        state.approved_claim_count += 1;
        state.denied_claim_count -= 1;
        state.approved_claim_amount += processed_claim.claim_amount;
        hospital.undenied_claim_count += 1;
        hospital.approved_claim_count += 1;
        hospital.approved_claim_amount += processed_claim.claim_amount;
        insurance_company.undenied_claim_count += 1;
        insurance_company.approved_claim_count += 1;
        insurance_company.approved_claim_amount += processed_claim.claim_amount;

        processed_claim.status = Status::Approved as u8;
        processed_claim.hospital_record_index = hospital.record_count;
        processed_claim.insurance_company_record_index = insurance_company.record_count;
        processed_claim.is_hospital_record_created = true;
        processed_claim.is_insurance_company_record_created = true;
        processed_claim.processed_time = time_stamp;

        let patient_record = &mut ctx.accounts.patient_record;
        patient_record.status = Status::Approved as u8;
        patient_record.patient_record_only = false;
        patient_record.processed_time = time_stamp;

        let hospital_record = &mut ctx.accounts.hospital_record;
        hospital.record_count += 1;
        hospital_record.record_id = hospital.record_count;
        hospital_record.claim_id = processed_claim.claim_id;
        hospital_record.status = Status::Approved as u8;
        hospital_record.submitter_address = processed_claim.submitter_address;
        hospital_record.patient_index = processed_claim.patient_index;
        hospital_record.processor_address = ctx.accounts.signer.key();
        hospital_record.insurance_company_index = processed_claim.insurance_company_index as u16;
        hospital_record.hospital_bill_invoice_number = processed_claim.hospital_bill_invoice_number.clone();
        hospital_record.claim_amount = processed_claim.claim_amount;
        hospital_record.ailment = processed_claim.ailment.clone();
        hospital_record.note = processed_claim.note.clone();
        hospital_record.submitted_time = processed_claim.submitted_time;
        hospital_record.processed_time = time_stamp;
        
        let insurance_company_record = &mut ctx.accounts.insurance_company_record;
        insurance_company.record_count += 1;
        insurance_company_record.record_id = insurance_company.record_count;
        insurance_company_record.claim_id = processed_claim.claim_id;
        insurance_company_record.status = Status::Approved as u8;
        insurance_company_record.submitter_address = processed_claim.submitter_address;
        insurance_company_record.patient_index = processed_claim.patient_index;
        insurance_company_record.processor_address = ctx.accounts.signer.key();
        insurance_company_record.country_index = processed_claim.country_index;
        insurance_company_record.state_index = processed_claim.state_index;
        insurance_company_record.hospital_index = processed_claim.hospital_index as u32;
        insurance_company_record.hospital_bill_invoice_number = processed_claim.hospital_bill_invoice_number.clone();
        insurance_company_record.claim_amount = processed_claim.claim_amount;
        insurance_company_record.ailment = processed_claim.ailment.clone();
        insurance_company_record.note = processed_claim.note.clone();
        insurance_company_record.submitted_time = processed_claim.submitted_time;
        insurance_company_record.processed_time = time_stamp;

        msg!("New Undenied Claim");
        msg!("New Hospital Record Created");
        msg!("New Insurance Company Record Created");
        msg!("Processed Claim Number: {}", processed_claim.processed_claim_id);

        Ok(())
    }

    pub fn undeny_claim_with_all_records(ctx: Context<UndenyClaimWithAllRecords>, _processor_address: Pubkey, _processor_count_index: u64) -> Result<()> 
    {
        let processed_claim = &mut ctx.accounts.processed_claim;

        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        //Only denied or appealed claims can be undenied
        require!((processed_claim.status == Status::Denied as u8) || (processed_claim.status == Status::Appealed as u8), InvalidOperationError::ClaimNotDeniedOrAppealed);

        let processor_stats = &mut ctx.accounts.processor_stats;
        let submitter = &mut ctx.accounts.submitter;
        let patient = &mut ctx.accounts.patient;
        let processor = &mut ctx.accounts.processor;
        let state = &mut ctx.accounts.state;
        let hospital = &mut ctx.accounts.hospital;
        let insurance_company = &mut ctx.accounts.insurance_company;
        let time_stamp = Clock::get()?.unix_timestamp as u64;
        
        processor_stats.approved_claim_amount += processed_claim.claim_amount;
        processor_stats.undenied_claim_count += 1;
        processor_stats.approved_claim_count += 1;
        processor_stats.denied_claim_count -= 1;
        submitter.undenied_claim_count += 1;
        submitter.approved_claim_count += 1;
        submitter.denied_claim_count -= 1;
        submitter.approved_claim_amount += processed_claim.claim_amount;
        patient.undenied_claim_count += 1;
        patient.approved_claim_count += 1;
        patient.denied_claim_count -= 1;
        patient.approved_claim_amount += processed_claim.claim_amount;
        processor.undenied_claim_count += 1;
        processor.approved_claim_amount += processed_claim.claim_amount;
        state.undenied_claim_count += 1;
        state.approved_claim_count += 1;
        state.denied_claim_count -= 1;
        state.approved_claim_amount += processed_claim.claim_amount;
        hospital.undenied_claim_count += 1;
        hospital.approved_claim_count += 1;
        hospital.denied_claim_count -= 1;
        hospital.approved_claim_amount += processed_claim.claim_amount;
        insurance_company.undenied_claim_count += 1;
        insurance_company.approved_claim_count += 1;
        insurance_company.denied_claim_count -= 1;
        insurance_company.approved_claim_amount += processed_claim.claim_amount;

        processed_claim.status = Status::Approved as u8;
        processed_claim.processed_time = time_stamp;

        let patient_record = &mut ctx.accounts.patient_record;
        patient_record.status = Status::Approved as u8;
        patient_record.processed_time = time_stamp;

        let hospital_record = &mut ctx.accounts.hospital_record;
        hospital_record.status = Status::Approved as u8;
        hospital_record.processed_time = time_stamp;

        let insurance_company_record = &mut ctx.accounts.insurance_company_record;
        insurance_company_record.status = Status::Approved as u8;
        insurance_company_record.processed_time = time_stamp;
        
        msg!("New Undenied Claim");
        msg!("Processed Claim Number: {}", processed_claim.processed_claim_id);

        Ok(())
    }

    pub fn edit_processed_claim_and_patient_record(ctx: Context<EditProcessedClaimAndPatientRecord>, 
        _processor_address: Pubkey,
        _processor_count_index: u64,
        hospital_index: u32,
        insurance_company_index: u16,
        hospital_bill_invoice_number: String,
        claim_note: String,
        claim_amount: u64,
        ailment: String) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        let processor_stats = &mut ctx.accounts.processor_stats;
        let processed_claim = &mut ctx.accounts.processed_claim;
        let patient = &mut ctx.accounts.patient;
        let hospital = &mut ctx.accounts.hospital;
        let insurance_company = &mut ctx.accounts.insurance_company;
        let time_stamp = Clock::get()?.unix_timestamp as u64;

        //An edit count is kept to help stream line the table listeners on the front end
        patient.edited_record_count += 1;
        processor_stats.edited_claim_or_processed_claim_count += 1;

        //Update Processed Claim
        processed_claim.hospital_index = hospital_index as i32;
        processed_claim.hospital_bill_invoice_number = hospital_bill_invoice_number.clone();
        processed_claim.note = claim_note.clone();
        processed_claim.claim_amount = claim_amount;
        processed_claim.ailment = ailment.clone();
        processed_claim.insurance_company_index = insurance_company_index as i16;
        processed_claim.hospital_name = hospital.hospital_name.clone();
        processed_claim.hospital_address = hospital.hospital_address.clone();
        processed_claim.hospital_city = hospital.hospital_city.clone();
        processed_claim.hospital_zip_code = hospital.hospital_zip_code;
        processed_claim.hospital_phone_number = hospital.hospital_phone_number.clone();
        processed_claim.insurance_company_name = insurance_company.insurance_company_name.clone();
        processed_claim.processed_time = time_stamp;

        //Update Records
        let patient_record = &mut ctx.accounts.patient_record;
        patient_record.hospital_index = hospital_index;
        patient_record.insurance_company_index = insurance_company_index;
        patient_record.hospital_bill_invoice_number = hospital_bill_invoice_number.clone();
        patient_record.claim_amount = claim_amount;
        patient_record.ailment = ailment.clone();
        patient_record.note = claim_note.clone();
        patient_record.processed_time = time_stamp;
        
        msg!("Processed Claim And Patient Record Updated");
        msg!("Processed Claim Number: {}", processed_claim.processed_claim_id);

        Ok(())
    }

    pub fn edit_processed_claim_and_all_records(ctx: Context<EditProcessedClaimAndAllRecords>, 
        _processor_address: Pubkey,
        _processor_count_index: u64,
        hospital_bill_invoice_number: String,
        claim_note: String,
        claim_amount: u64,
        ailment: String) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        let processor_stats = &mut ctx.accounts.processor_stats;
        let submitter = &mut ctx.accounts.submitter;
        let patient = &mut ctx.accounts.patient;
        let processor = &mut ctx.accounts.processor;
        let state = &mut ctx.accounts.state;
        let hospital = &mut ctx.accounts.hospital;
        let insurance_company = &mut ctx.accounts.insurance_company;
        let processed_claim = &mut ctx.accounts.processed_claim;
        let time_stamp = Clock::get()?.unix_timestamp as u64;

        //An edit count is kept to help stream line the table listeners on the front end
        patient.edited_record_count += 1;
        hospital.edited_record_count += 1;
        insurance_company.edited_record_count += 1;
        processor_stats.edited_claim_or_processed_claim_count += 1;

        //Update Previous Amounts If Amounts Were Already Approved
        if processed_claim.status == Status::Approved as u8
        {
            processor_stats.approved_claim_amount -= processed_claim.claim_amount;
            processor_stats.approved_claim_amount += claim_amount;
            submitter.approved_claim_amount -= processed_claim.claim_amount;
            submitter.approved_claim_amount += claim_amount;
            patient.approved_claim_amount -= processed_claim.claim_amount;
            patient.approved_claim_amount += claim_amount;
            processor.approved_claim_amount -= processed_claim.claim_amount;
            processor.approved_claim_amount += claim_amount;
            hospital.approved_claim_amount -= processed_claim.claim_amount;
            hospital.approved_claim_amount += claim_amount;
            state.approved_claim_amount -= processed_claim.claim_amount;
            state.approved_claim_amount += claim_amount;
            insurance_company.approved_claim_amount -= processed_claim.claim_amount;
            insurance_company.approved_claim_amount += claim_amount;
        }

        //Update Processed Claim
        processed_claim.hospital_bill_invoice_number = hospital_bill_invoice_number.clone();
        processed_claim.note = claim_note.clone();
        processed_claim.claim_amount = claim_amount;
        processed_claim.ailment = ailment.clone();
        processed_claim.processed_time = time_stamp;

        //Update Records
        let patient_record = &mut ctx.accounts.patient_record;
        patient_record.hospital_bill_invoice_number = hospital_bill_invoice_number.clone();
        patient_record.claim_amount = claim_amount;
        patient_record.ailment = ailment.clone();
        patient_record.note = claim_note.clone();
        patient_record.processed_time = time_stamp;

        let hospital_record = &mut ctx.accounts.hospital_record;
        hospital_record.hospital_bill_invoice_number = hospital_bill_invoice_number.clone();
        hospital_record.claim_amount = claim_amount; 
        hospital_record.ailment = ailment.clone();
        hospital_record.note = claim_note.clone();
        hospital_record.processed_time = time_stamp;

        let insurance_company_record = &mut ctx.accounts.insurance_company_record;
        insurance_company_record.hospital_bill_invoice_number = hospital_bill_invoice_number.clone();
        insurance_company_record.claim_amount = claim_amount;
        insurance_company_record.ailment = ailment;
        insurance_company_record.note = claim_note;
        insurance_company_record.processed_time = time_stamp;

        msg!("Processed Claim And All Records Updated");
        msg!("Processed Claim Number: {}", processed_claim.processed_claim_id);

        Ok(())
    }

    pub fn revoke_approval(ctx: Context<RevokeApproval>, _processor_address: Pubkey, _processor_count_index: u64, denial_reason: String) -> Result<()> 
    {
        let processed_claim = &mut ctx.accounts.processed_claim;

        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        //Only approved claims can be revoked
        require!(processed_claim.status == Status::Approved as u8, InvalidOperationError::ClaimNotApproved);

        //Denial note string must not be longer than 140 characters
        require!(denial_reason.len() <= MAX_NOTE_LENGTH, InvalidLengthError::NoteTooLong);

        let processor_stats = &mut ctx.accounts.processor_stats;
        let submitter = &mut ctx.accounts.submitter;
        let patient = &mut ctx.accounts.patient;
        let processor = &mut ctx.accounts.processor;
        let state = &mut ctx.accounts.state;
        let hospital = &mut ctx.accounts.hospital;
        let insurance_company = &mut ctx.accounts.insurance_company;
        let time_stamp = Clock::get()?.unix_timestamp as u64;
        
        processor_stats.approved_claim_amount -= processed_claim.claim_amount;
        processor_stats.revoked_approval_count += 1;
        processor_stats.approved_claim_count -= 1;
        processor_stats.denied_claim_count += 1;
        submitter.revoked_approval_count += 1;
        submitter.approved_claim_count -= 1;
        submitter.denied_claim_count += 1;
        submitter.approved_claim_amount -= processed_claim.claim_amount;
        patient.revoked_approval_count += 1;
        patient.approved_claim_count -= 1;
        patient.denied_claim_count += 1;
        patient.approved_claim_amount -= processed_claim.claim_amount;
        processor.revoked_approval_count += 1;
        processor.approved_claim_amount -= processed_claim.claim_amount;
        state.revoked_approval_count += 1;
        state.approved_claim_count -= 1;
        state.denied_claim_count += 1;
        state.approved_claim_amount -= processed_claim.claim_amount;
        hospital.revoked_approval_count += 1;
        hospital.approved_claim_count -= 1;
        hospital.denied_claim_count += 1;
        hospital.approved_claim_amount -= processed_claim.claim_amount;
        insurance_company.revoked_approval_count += 1;
        insurance_company.approved_claim_count -= 1;
        insurance_company.denied_claim_count += 1;
        insurance_company.approved_claim_amount -= processed_claim.claim_amount;

        processed_claim.status = Status::Denied as u8;
        processed_claim.denial_reason = denial_reason.clone();
        processed_claim.processed_time = time_stamp;

        let patient_record = &mut ctx.accounts.patient_record;
        patient_record.status = Status::Denied as u8;
        patient_record.denial_reason = denial_reason.clone();
        patient_record.processed_time = time_stamp;

        let hospital_record = &mut ctx.accounts.hospital_record;
        hospital_record.status = Status::Denied as u8;
        hospital_record.denial_reason = denial_reason.clone();
        hospital_record.processed_time = time_stamp;

        let insurance_company_record = &mut ctx.accounts.insurance_company_record;
        insurance_company_record.status = Status::Denied as u8;
        insurance_company_record.denial_reason = denial_reason.clone();
        insurance_company_record.processed_time = time_stamp;
        
        msg!("New Revoked Approval");
        msg!("Processed Claim Number: {}", processed_claim.processed_claim_id);

        Ok(())
    }

    pub fn drop_denial_hammer(ctx: Context<DropDenialHammer>) -> Result<()> 
    {
        let ceo = &mut ctx.accounts.ceo;
        //Only the CEO can call this function
        require_keys_eq!(ctx.accounts.signer.key(), ceo.address.key(), AuthorizationError::NotCEO);

        for claim_account in ctx.remaining_accounts.iter()
        {
            //Transfer tokens from the account to the sol_destination.
            let dest_starting_lamports = ctx.accounts.signer.lamports();
            **ctx.accounts.signer.lamports.borrow_mut() = 
                dest_starting_lamports.checked_add(claim_account.lamports()).unwrap();
            **claim_account.lamports.borrow_mut() = 0;
            
            claim_account.assign(&system_program::ID);
            let _ = claim_account.realloc(0, false);
        }

        let processor_stats = &mut ctx.accounts.processor_stats;
        let claim_queue = &mut ctx.accounts.claim_queue;
        let processor = &mut ctx.accounts.processor;

        processor_stats.denial_hammer_dropped_count += 1;
        claim_queue.current_claim_queue_count = claim_queue.current_claim_queue_count - ctx.remaining_accounts.len() as u32;
        processor.denial_hammer_dropped_count += 1;
        
        msg!("Denial Hammer Dropped");
        msg!("Denial Hammer Use Count: {}", processor_stats.denial_hammer_dropped_count);
        msg!("Number of Accounts Hammered: {}", ctx.remaining_accounts.len());

        Ok(())
    }
}

//Derived Accounts
#[derive(Accounts)]
pub struct InitializeAdminAccounts<'info> 
{
    #[account(
        init, 
        payer = signer,
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump,
        space = size_of::<M4AProtocolCEO>() + 8)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        init, 
        payer = signer,
        seeds = [b"m4aProtocolTreasurer".as_ref()],
        bump,
        space = size_of::<M4AProtocolTreasurer>() + 8)]
    pub treasurer: Account<'info, M4AProtocolTreasurer>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct PassOnM4AProtocolCEO<'info> 
{
    #[account(
        mut,
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct PassOnM4AProtocolTreasurer<'info> 
{
    #[account(
        mut,
        seeds = [b"m4aProtocolTreasurer".as_ref()],
        bump)]
    pub treasurer: Account<'info, M4AProtocolTreasurer>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(token_mint_address: Pubkey)]
pub struct AddFeeTokenEntry<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        init, 
        payer = signer, 
        seeds = [b"feeTokenEntry".as_ref(),
        token_mint_address.key().as_ref()], 
        bump, 
        space = size_of::<FeeTokenEntry>() + 8)]
    pub fee_token_entry: Account<'info, FeeTokenEntry>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(token_mint_address: Pubkey)]
pub struct RemoveFeeTokenEntry<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut,
        close = signer,
        seeds = [b"feeTokenEntry".as_ref(),
        token_mint_address.key().as_ref()], 
        bump)]
    pub fee_token_entry: Account<'info, FeeTokenEntry>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct InitializeProtocolStats<'info>
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        init, 
        payer = signer,
        seeds = [b"processorStats".as_ref()],
        bump,
        space = size_of::<ProcessorStats>() + 8)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        init, 
        payer = signer,
        seeds = [b"hospitalStats".as_ref()],
        bump,
        space = size_of::<HospitalStats>() + 8)]
    pub hospital_stats: Account<'info, HospitalStats>,

    #[account(
        init, 
        payer = signer,
        seeds = [b"insuranceCompanyStats".as_ref()],
        bump,
        space = size_of::<InsuranceCompanyStats>() + 8)]
    pub insurance_company_stats: Account<'info, InsuranceCompanyStats>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct InitializeM4AProtocolAndClaimQueue<'info> 
{
    #[account(
        init, 
        payer = signer,
        seeds = [b"m4aProtocol".as_ref()],
        bump,
        space = size_of::<M4AProtocol>() + 8)]
    pub m4a_protocol: Account<'info, M4AProtocol>,

    //Stats account must exist to initialize protocol
    #[account(
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        init, 
        payer = signer,
        seeds = [b"claimQueue".as_ref()],
        bump,
        space = size_of::<ClaimQueue>() + 8)]
    pub claim_queue: Account<'info, ClaimQueue>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct SetClaimQueueFlag<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut,
        seeds = [b"claimQueue".as_ref()],
        bump)]
    pub claim_queue: Account<'info, ClaimQueue>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct EditClaimQueueSize<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,
    
    #[account(
        mut, 
        seeds = [b"claimQueue".as_ref()],
        bump)]
    pub claim_queue: Account<'info, ClaimQueue>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct CreateSubmitterAccount<'info> 
{
    #[account(
        mut,
        seeds = [b"m4aProtocol".as_ref()],
        bump)]
    pub m4a_protocol: Account<'info, M4AProtocol>,

    #[account(
        init, 
        payer = signer,
        seeds = [b"submitter".as_ref(), signer.key().as_ref()],
        bump,
        space = size_of::<SubmitterAccount>() + 8)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct CreatePatientAccount<'info> 
{
    #[account(
        mut,
        seeds = [b"m4aProtocol".as_ref()],
        bump)]
    pub m4a_protocol: Account<'info, M4AProtocol>,

    #[account(
        mut,
        seeds = [b"submitter".as_ref(), signer.key().as_ref()],
        bump)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(
        init,
        payer = signer,
        seeds = [b"patient".as_ref(), signer.key().as_ref(), submitter.patient_count.to_le_bytes().as_ref()],
        bump,
        space = size_of::<PatientAccount>() + PATIENT_EXTRA_SIZE + 8)]
    pub patient: Account<'info, PatientAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(patient_index: u8)]
pub struct SetPatientFlag<'info> 
{
    #[account(
        mut,
        seeds = [b"submitter".as_ref(), signer.key().as_ref()],
        bump)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(
        mut,
        seeds = [b"patient".as_ref(), signer.key().as_ref(), patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(processor_address: Pubkey)]
pub struct CreateProcessorAccount<'info>
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut,
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        init, 
        payer = signer,
        seeds = [b"processor".as_ref(), processor_address.key().as_ref()],
        bump,
        space = size_of::<ProcessorAccount>() + 8)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(processor_address: Pubkey)]
pub struct SetProcessorAccountActiveFlag<'info>
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut,
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), processor_address.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(processor_address: Pubkey)]
pub struct SetProcessorAccountPrivilege<'info>
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut,
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), processor_address.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(patient_index: u8, token_mint_address: Pubkey)]
pub struct SubmitClaimToQueue<'info> 
{
    #[account(
        mut,
        seeds = [b"submitter".as_ref(), signer.key().as_ref()],
        bump)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(
        seeds = [b"patient".as_ref(), signer.key().as_ref(), patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut,
        seeds = [b"claimQueue".as_ref()],
        bump)]
    pub claim_queue: Account<'info, ClaimQueue>,
    
    #[account(
        init, 
        payer = signer,
        seeds = [b"claim".as_ref(), signer.key().as_ref()], 
        bump, 
        space = size_of::<Claim>() + CLAIM_EXTRA_SIZE + 8)]
    pub claim: Account<'info, Claim>,

    #[account(
        seeds = [b"m4aProtocolTreasurer".as_ref()],
        bump)]
    pub treasurer: Account<'info, M4AProtocolTreasurer>,

    #[account(
        mut,
        associated_token::mint = fee_token_entry.token_mint_address,
        associated_token::authority = signer
    )]
    pub user_fee_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = fee_token_entry.token_mint_address,
        associated_token::authority = treasurer.address
        //address = anchor_spl::associated_token::get_associated_token_address(&treasurer.address, &USDC_MINT)
    )]
    pub treasurer_usdc_ata: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"feeTokenEntry".as_ref(),
        token_mint_address.key().as_ref()], 
        bump)]
    pub fee_token_entry: Account<'info, FeeTokenEntry>,

    pub token_program: Program<'info, Token>,
    
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(submitter_address: Pubkey)]
pub struct AssignClaimToProcessor<'info> 
{
    #[account(
        mut,
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        mut,
        seeds = [b"claim".as_ref(), submitter_address.key().as_ref()], 
        bump)]
    pub claim: Account<'info, Claim>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(submitter_address: Pubkey)]
pub struct ReassignClaimToNewProcessor<'info> 
{
    #[account(
        mut,
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut,
        seeds = [b"claim".as_ref(), submitter_address.key().as_ref()], 
        bump)]
    pub claim: Account<'info, Claim>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub new_processor: Account<'info, ProcessorAccount>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), claim.processor_address.key().as_ref()],
        bump)]
    pub old_processor: Account<'info, ProcessorAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(submitter_address: Pubkey)]
pub struct UnassignClaimFromProcessor<'info> 
{
    #[account(
        mut,
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut,
        seeds = [b"claim".as_ref(), submitter_address.key().as_ref()], 
        bump)]
    pub claim: Account<'info, Claim>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub admin_processor: Account<'info, ProcessorAccount>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), claim.processor_address.key().as_ref()],
        bump)]
    pub old_processor: Account<'info, ProcessorAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

//In the event that the claim has already been denied some kind of way and the processor is stuck on a dead claim
#[derive(Accounts)]
#[instruction(processor_address: Pubkey)]
pub struct SetProcessorToNotProcessingClaimState<'info> 
{
    #[account(
        mut,
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub admin_processor: Account<'info, ProcessorAccount>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), processor_address.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(submitter_address: Pubkey, country_index: u16, state_index: u32)]
pub struct CreateStateAccount<'info> 
{
    #[account(
        mut,
        seeds = [b"m4aProtocol".as_ref()],
        bump)]
    pub m4a_protocol: Account<'info, M4AProtocol>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(
        init, 
        payer = signer,
        seeds = [b"state".as_ref(), country_index.to_le_bytes().as_ref(), state_index.to_le_bytes().as_ref()],
        bump,
        space = size_of::<StateAccount>() + 8)]
    pub state: Account<'info, StateAccount>,

    #[account(
        mut,
        seeds = [b"claim".as_ref(), submitter_address.key().as_ref()], 
        bump)]
    pub claim: Account<'info, Claim>, 

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(submitter_address: Pubkey, country_index: u16, state_index: u32)]
pub struct CreateHospital<'info> 
{
    #[account(
        mut,
        seeds = [b"hospitalStats".as_ref()],
        bump)]
    pub hospital_stats: Account<'info, HospitalStats>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(
        mut,
        seeds = [b"claim".as_ref(), submitter_address.key().as_ref()], 
        bump)]
    pub claim: Account<'info, Claim>, 

    #[account(
        mut, 
        seeds = [b"state".as_ref(), country_index.to_le_bytes().as_ref(), state_index.to_le_bytes().as_ref()],
        bump)]
    pub state: Account<'info, StateAccount>,

    #[account(
        init, 
        payer = signer,
        seeds = [b"hospital".as_ref(), country_index.to_le_bytes().as_ref(), state_index.to_le_bytes().as_ref(), state.hospital_count.to_le_bytes().as_ref()],
        bump,
        space = size_of::<Hospital>() + HOSPITAL_EXTRA_SIZE + 8)]
    pub hospital: Account<'info, Hospital>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(country_index: u16, state_index: u32, hospital_index: u32)]
pub struct EditHospital<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut,
        seeds = [b"hospitalStats".as_ref()],
        bump)]
    pub hospital_stats: Account<'info, HospitalStats>,

    #[account(
        mut, 
        seeds = [b"state".as_ref(), country_index.to_le_bytes().as_ref(), state_index.to_le_bytes().as_ref()],
        bump)]
    pub state: Account<'info, StateAccount>,

    #[account(
        mut,
        seeds = [b"hospital".as_ref(), country_index.to_le_bytes().as_ref(), state_index.to_le_bytes().as_ref(), hospital_index.to_le_bytes().as_ref()],
        bump)]
    pub hospital: Account<'info, Hospital>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(submitter_address: Pubkey, insurance_company_index: u16)]
pub struct CreateInsuranceCompany<'info> 
{
    #[account(
        mut,
        seeds = [b"insuranceCompanyStats".as_ref()],
        bump)]
    pub insurance_company_stats: Account<'info, InsuranceCompanyStats>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(
        mut,
        seeds = [b"claim".as_ref(), submitter_address.key().as_ref()], 
        bump)]
    pub claim: Account<'info, Claim>, 

    #[account(
        init, 
        payer = signer,
        seeds = [b"insuranceCompany".as_ref(), insurance_company_index.to_le_bytes().as_ref()],
        bump,
        space = size_of::<InsuranceCompany>() + INSURANCE_COMPANY_EXTRA_SIZE + 8)]
    pub insurance_company: Account<'info, InsuranceCompany>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(insurance_company_index: u16)]
pub struct EditInsuranceCompany<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut,
        seeds = [b"insuranceCompanyStats".as_ref()],
        bump)]
    pub insurance_company_stats: Account<'info, InsuranceCompanyStats>,

    #[account(
        mut, 
        seeds = [b"insuranceCompany".as_ref(), insurance_company_index.to_le_bytes().as_ref()],
        bump)]
    pub insurance_company: Account<'info, InsuranceCompany>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(submitter_address: Pubkey)]
pub struct UpdateClaim<'info> 
{
    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        mut,
        seeds = [b"claim".as_ref(), submitter_address.key().as_ref()], 
        bump)]
    pub claim: Account<'info, Claim>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), claim.processor_address.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(submitter_address: Pubkey)]
pub struct CreatePatientRecord<'info> 
{
    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(
        mut,
        seeds = [b"claim".as_ref(), submitter_address.key().as_ref()], 
        bump)]
    pub claim: Account<'info, Claim>, 

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), claim.submitter_address.key().as_ref(), claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        init, 
        payer = signer,
        seeds = [b"patientRecord".as_ref(), claim.submitter_address.key().as_ref(), claim.patient_index.to_le_bytes().as_ref(), patient.record_count.to_le_bytes().as_ref()], 
        bump, 
        space = size_of::<PatientRecord>() + PATIENT_RECORD_EXTRA_SIZE + 8)]
    pub patient_record: Account<'info, PatientRecord>,  

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(submitter_address: Pubkey)]
pub struct CreateHospitalAndInsuranceCompanyRecords<'info> 
{
    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Box<Account<'info, ProcessorStats>>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(
        mut,
        seeds = [b"claim".as_ref(), submitter_address.key().as_ref()], 
        bump)]
    pub claim: Box<Account<'info, Claim>>,

    #[account(
        mut, 
        seeds = [b"patientRecord".as_ref(), claim.submitter_address.key().as_ref(), claim.patient_index.to_le_bytes().as_ref(), claim.patient_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub patient_record: Account<'info, PatientRecord>,  

    #[account(
        mut, 
        seeds = [b"hospital".as_ref(), claim.country_index.to_le_bytes().as_ref(), claim.state_index.to_le_bytes().as_ref(), claim.hospital_index.to_le_bytes().as_ref()],
        bump)]
    pub hospital: Account<'info, Hospital>,

    #[account(
        init, 
        payer = signer,
        seeds = [b"hospitalRecord".as_ref(),
        claim.country_index.to_le_bytes().as_ref(),
        claim.state_index.to_le_bytes().as_ref(),
        claim.hospital_index.to_le_bytes().as_ref(),
        hospital.record_count.to_le_bytes().as_ref()], 
        bump, 
        space = size_of::<HospitalRecord>() + HOSPITAL_RECORD_EXTRA_SIZE + 8)]
    pub hospital_record: Account<'info, HospitalRecord>, 

    #[account(
        mut, 
        seeds = [b"insuranceCompany".as_ref(), claim.insurance_company_index.to_le_bytes().as_ref()],
        bump)]
    pub insurance_company: Box<Account<'info, InsuranceCompany>>,

    #[account(
        init, 
        payer = signer,
        seeds = [b"insuranceCompanyRecord".as_ref(), claim.insurance_company_index.to_le_bytes().as_ref(), insurance_company.record_count.to_le_bytes().as_ref()], 
        bump, 
        space = size_of::<InsuranceCompanyRecord>() + INSURANCE_COMPANY_RECORD_EXTRA_SIZE + 8)]
    pub insurance_company_record: Account<'info, InsuranceCompanyRecord>,  

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(_submitter_address: Pubkey)]
pub struct ApproveClaim<'info> 
{
    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        mut,
        seeds = [b"claimQueue".as_ref()],
        bump)]
    pub claim_queue: Account<'info, ClaimQueue>,

    #[account(
        mut, 
        seeds = [b"submitter".as_ref(), claim.submitter_address.key().as_ref()],
        bump)]
    pub submitter: Box<Account<'info, SubmitterAccount>>,

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), claim.submitter_address.key().as_ref(), claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Box<Account<'info, ProcessorAccount>>,

    #[account(
        mut, 
        seeds = [b"state".as_ref(), claim.country_index.to_le_bytes().as_ref(), claim.state_index.to_le_bytes().as_ref()],
        bump)]
    pub state: Box<Account<'info, StateAccount>>,

    #[account(
        mut, 
        seeds = [b"patientRecord".as_ref(), claim.submitter_address.key().as_ref(), claim.patient_index.to_le_bytes().as_ref(), claim.patient_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub patient_record: Account<'info, PatientRecord>,  

    #[account(
        mut,
        seeds = [b"hospital".as_ref(), claim.country_index.to_le_bytes().as_ref(), claim.state_index.to_le_bytes().as_ref(), claim.hospital_index.to_le_bytes().as_ref()], 
        bump)]
    pub hospital: Box<Account<'info, Hospital>>,  

    #[account(
        mut, 
        seeds = [b"hospitalRecord".as_ref(), claim.country_index.to_le_bytes().as_ref(), claim.state_index.to_le_bytes().as_ref(), claim.hospital_index.to_le_bytes().as_ref(), claim.hospital_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub hospital_record: Box<Account<'info, HospitalRecord>>,  

    #[account(
        mut,
        seeds = [b"insuranceCompany".as_ref(), claim.insurance_company_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company: Box<Account<'info, InsuranceCompany>>,  

    #[account(
        mut, 
        seeds = [b"insuranceCompanyRecord".as_ref(), claim.insurance_company_index.to_le_bytes().as_ref(), claim.insurance_company_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company_record: Box<Account<'info, InsuranceCompanyRecord>>,  
    
    #[account(
        init, 
        payer = signer,
        seeds = [b"processedClaim".as_ref(), signer.key().as_ref(), processor.processed_claim_count.to_le_bytes().as_ref()], 
        bump, 
        space = size_of::<ProcessedClaim>() + PROCESSED_CLAIM_EXTRA_SIZE + 8)]
    pub processed_claim: Account<'info, ProcessedClaim>,  

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,

    #[account(
        mut,
        close = signer,
        seeds = [b"claim".as_ref(), _submitter_address.key().as_ref()], 
        bump)]
    pub claim: Box<Account<'info, Claim>>, 
}

#[derive(Accounts)]
#[instruction(submitter_address: Pubkey)]
pub struct ApproveClaimWithEdits<'info> 
{
    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Box<Account<'info, ProcessorStats>>,

    #[account(
        mut,
        seeds = [b"claimQueue".as_ref()],
        bump)]
    pub claim_queue: Account<'info, ClaimQueue>,

    #[account(
        mut, 
        seeds = [b"submitter".as_ref(), claim.submitter_address.key().as_ref()],
        bump)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), claim.submitter_address.key().as_ref(), claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Box<Account<'info, ProcessorAccount>>,

    #[account(
        mut, 
        seeds = [b"state".as_ref(), claim.country_index.to_le_bytes().as_ref(), claim.state_index.to_le_bytes().as_ref()],
        bump)]
    pub state: Box<Account<'info, StateAccount>>,

    #[account(
        mut,
        close = signer,
        seeds = [b"claim".as_ref(), submitter_address.key().as_ref()], 
        bump)]
    pub claim: Box<Account<'info, Claim>>,     

    #[account(
        mut, 
        seeds = [b"patientRecord".as_ref(), claim.submitter_address.key().as_ref(), claim.patient_index.to_le_bytes().as_ref(), claim.patient_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub patient_record: Account<'info, PatientRecord>,  
    
    #[account(
        mut,
        seeds = [b"hospital".as_ref(), claim.country_index.to_le_bytes().as_ref(), claim.state_index.to_le_bytes().as_ref(), claim.hospital_index.to_le_bytes().as_ref()], 
        bump)]
    pub hospital: Box<Account<'info, Hospital>>,  

    #[account(
        mut, 
        seeds = [b"hospitalRecord".as_ref(), claim.country_index.to_le_bytes().as_ref(), claim.state_index.to_le_bytes().as_ref(), claim.hospital_index.to_le_bytes().as_ref(), claim.hospital_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub hospital_record: Box<Account<'info, HospitalRecord>>,  

    #[account(
        mut,
        seeds = [b"insuranceCompany".as_ref(), claim.insurance_company_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company: Box<Account<'info, InsuranceCompany>>,  

    #[account(
        mut, 
        seeds = [b"insuranceCompanyRecord".as_ref(), claim.insurance_company_index.to_le_bytes().as_ref(), claim.insurance_company_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company_record: Box<Account<'info, InsuranceCompanyRecord>>,
    
    #[account(
        init, 
        payer = signer,
        seeds = [b"processedClaim".as_ref(), signer.key().as_ref(), processor.processed_claim_count.to_le_bytes().as_ref()], 
        bump, 
        space = size_of::<ProcessedClaim>() + PROCESSED_CLAIM_EXTRA_SIZE + 8)]
    pub processed_claim: Account<'info, ProcessedClaim>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(submitter_address: Pubkey)]
pub struct CreatePatientRecordAndDenyClaim<'info> 
{
    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Box<Account<'info, ProcessorStats>>,

    #[account(
        mut,
        seeds = [b"claimQueue".as_ref()],
        bump)]
    pub claim_queue: Box<Account<'info, ClaimQueue>>,

    #[account(
        mut, 
        seeds = [b"submitter".as_ref(), claim.submitter_address.key().as_ref()],
        bump)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), claim.submitter_address.key().as_ref(), claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Box<Account<'info, ProcessorAccount>>,

    #[account(
        mut, 
        seeds = [b"state".as_ref(), claim.country_index.to_le_bytes().as_ref(), claim.state_index.to_le_bytes().as_ref()],
        bump)]
    pub state: Box<Account<'info, StateAccount>>,

    #[account(
        init, 
        payer = signer,  
        seeds = [b"patientRecord".as_ref(), claim.submitter_address.key().as_ref(), claim.patient_index.to_le_bytes().as_ref(), patient.record_count.to_le_bytes().as_ref()], 
        bump, 
        space = size_of::<PatientRecord>() + PATIENT_RECORD_EXTRA_SIZE + 8)]
    pub patient_record: Account<'info, PatientRecord>,  
    
    #[account(
        init, 
        payer = signer,
        seeds = [b"processedClaim".as_ref(), signer.key().as_ref(), processor.processed_claim_count.to_le_bytes().as_ref()], 
        bump, 
        space = size_of::<ProcessedClaim>() + PROCESSED_CLAIM_EXTRA_SIZE + 8)]
    pub processed_claim: Account<'info, ProcessedClaim>,  

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,

    #[account(
        mut,
        close = signer,
        seeds = [b"claim".as_ref(), submitter_address.key().as_ref()], 
        bump)]
    pub claim: Box<Account<'info, Claim>>
}

#[derive(Accounts)]
#[instruction(submitter_address: Pubkey)]
pub struct MaxDenyPendingClaim<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        mut,
        seeds = [b"claimQueue".as_ref()],
        bump)]
    pub claim_queue: Account<'info, ClaimQueue>,

    #[account(
        mut, 
        seeds = [b"submitter".as_ref(), claim.submitter_address.key().as_ref()],
        bump)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), claim.submitter_address.key().as_ref(), claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub admin_processor: Account<'info, ProcessorAccount>,

    #[account(
        mut,
        close = signer,
        seeds = [b"claim".as_ref(), submitter_address.key().as_ref()], 
        bump)]
    pub claim: Account<'info, Claim>, 

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(submitter_address: Pubkey)]
pub struct MaxDenyInProgressClaim<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        mut,
        seeds = [b"claimQueue".as_ref()],
        bump)]
    pub claim_queue: Account<'info, ClaimQueue>,

    #[account(
        mut, 
        seeds = [b"submitter".as_ref(), claim.submitter_address.key().as_ref()],
        bump)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), claim.submitter_address.key().as_ref(), claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub admin_processor: Account<'info, ProcessorAccount>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), claim.processor_address.key().as_ref()],
        bump)]
    pub claim_processor: Account<'info, ProcessorAccount>,

    #[account(
        mut,
        close = signer,
        seeds = [b"claim".as_ref(), submitter_address.key().as_ref()], 
        bump)]
    pub claim: Account<'info, Claim>, 

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(submitter_address: Pubkey)]
pub struct DenyClaimWithAllRecords<'info> 
{
    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Box<Account<'info, ProcessorStats>>,

    #[account(
        mut,
        seeds = [b"claimQueue".as_ref()],
        bump)]
    pub claim_queue: Account<'info, ClaimQueue>,

    #[account(
        mut, 
        seeds = [b"submitter".as_ref(), claim.submitter_address.key().as_ref()],
        bump)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), claim.submitter_address.key().as_ref(), claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Box<Account<'info, ProcessorAccount>>,

    #[account(
        mut, 
        seeds = [b"state".as_ref(), claim.country_index.to_le_bytes().as_ref(), claim.state_index.to_le_bytes().as_ref()],
        bump)]
    pub state: Box<Account<'info, StateAccount>>,

    #[account(
        mut, 
        seeds = [b"patientRecord".as_ref(), claim.submitter_address.key().as_ref(), claim.patient_index.to_le_bytes().as_ref(), claim.patient_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub patient_record: Account<'info, PatientRecord>,  

    #[account(
        mut, 
        seeds = [b"hospital".as_ref(), claim.country_index.to_le_bytes().as_ref(), claim.state_index.to_le_bytes().as_ref(), claim.hospital_index.to_le_bytes().as_ref()],
        bump)]
    pub hospital: Box<Account<'info, Hospital>>,

    #[account(
        mut, 
        seeds = [b"hospitalRecord".as_ref(), claim.country_index.to_le_bytes().as_ref(), claim.state_index.to_le_bytes().as_ref(), claim.hospital_index.to_le_bytes().as_ref(), claim.hospital_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub hospital_record: Box<Account<'info, HospitalRecord>>,  

    #[account(
        mut,
        seeds = [b"insuranceCompany".as_ref(), claim.insurance_company_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company: Box<Account<'info, InsuranceCompany>>,  

    #[account(
        mut, 
        seeds = [b"insuranceCompanyRecord".as_ref(), claim.insurance_company_index.to_le_bytes().as_ref(), claim.insurance_company_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company_record: Box<Account<'info, InsuranceCompanyRecord>>,
    
    #[account(
        init, 
        payer = signer,
        seeds = [b"processedClaim".as_ref(), signer.key().as_ref(), processor.processed_claim_count.to_le_bytes().as_ref()], 
        bump, 
        space = size_of::<ProcessedClaim>() + PROCESSED_CLAIM_EXTRA_SIZE + 8)]
    pub processed_claim: Account<'info, ProcessedClaim>,  

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,

    #[account(
        mut,
        close = signer,
        seeds = [b"claim".as_ref(), submitter_address.key().as_ref()], 
        bump)]
    pub claim: Box<Account<'info, Claim>>, 
}

#[derive(Accounts)]
#[instruction(processor_address: Pubkey, processor_count_index: u64, token_mint_address: Pubkey)]
pub struct AppealDeniedClaimWithOnlyPatientRecord<'info> 
{
    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        mut, 
        seeds = [b"submitter".as_ref(), processed_claim.submitter_address.key().as_ref()],
        bump)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut, 
        seeds = [b"state".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref()],
        bump)]
    pub state: Account<'info, StateAccount>,

    #[account(
        mut, 
        seeds = [b"patientRecord".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref(), processed_claim.patient_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub patient_record: Account<'info, PatientRecord>,

    #[account(
        mut, 
        seeds = [b"processedClaim".as_ref(), processor_address.key().as_ref(), processor_count_index.to_le_bytes().as_ref()], 
        bump)]
    pub processed_claim: Account<'info, ProcessedClaim>,

    #[account(
        seeds = [b"m4aProtocolTreasurer".as_ref()],
        bump)]
    pub treasurer: Account<'info, M4AProtocolTreasurer>,

    #[account(
        mut,
        associated_token::mint = fee_token_entry.token_mint_address,
        associated_token::authority = signer
    )]
    pub user_fee_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = fee_token_entry.token_mint_address,
        associated_token::authority = treasurer.address
        //address = anchor_spl::associated_token::get_associated_token_address(&treasurer.address, &USDC_MINT)
    )]
    pub treasurer_usdc_ata: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"feeTokenEntry".as_ref(),
        token_mint_address.key().as_ref()], 
        bump)]
    pub fee_token_entry: Account<'info, FeeTokenEntry>,

    pub token_program: Program<'info, Token>,
    
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(processor_address: Pubkey, processor_count_index: u64)]
pub struct DenyAppealedClaimWithOnlyPatientRecord<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        mut, 
        seeds = [b"submitter".as_ref(), processed_claim.submitter_address.key().as_ref()],
        bump)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(
        mut, 
        seeds = [b"state".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref()],
        bump)]
    pub state: Account<'info, StateAccount>,

    #[account(
        mut, 
        seeds = [b"patientRecord".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref(), processed_claim.patient_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub patient_record: Account<'info, PatientRecord>,

    #[account(
        mut, 
        seeds = [b"processedClaim".as_ref(), processor_address.key().as_ref(), processor_count_index.to_le_bytes().as_ref()], 
        bump)]
    pub processed_claim: Account<'info, ProcessedClaim>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(processor_address: Pubkey, processor_count_index: u64, token_mint_address: Pubkey)]
pub struct AppealDeniedClaimWithAllRecords<'info> 
{
    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Box<Account<'info, ProcessorStats>>,

    #[account(
        mut, 
        seeds = [b"submitter".as_ref(), processed_claim.submitter_address.key().as_ref()],
        bump)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut, 
        seeds = [b"state".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref()],
        bump)]
    pub state: Box<Account<'info, StateAccount>>,

    #[account(
        mut, 
        seeds = [b"patientRecord".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref(), processed_claim.patient_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub patient_record: Account<'info, PatientRecord>,  

    #[account(
        mut, 
        seeds = [b"hospital".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref(), processed_claim.hospital_index.to_le_bytes().as_ref()],
        bump)]
    pub hospital: Account<'info, Hospital>,

    #[account(
        mut, 
        seeds = [b"hospitalRecord".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref(), processed_claim.hospital_index.to_le_bytes().as_ref(), processed_claim.hospital_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub hospital_record: Account<'info, HospitalRecord>,  

    #[account(
        mut,
        seeds = [b"insuranceCompany".as_ref(), processed_claim.insurance_company_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company: Box<Account<'info, InsuranceCompany>>,  

    #[account(
        mut, 
        seeds = [b"insuranceCompanyRecord".as_ref(), processed_claim.insurance_company_index.to_le_bytes().as_ref(), processed_claim.insurance_company_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company_record: Account<'info, InsuranceCompanyRecord>,

    #[account(
        mut, 
        seeds = [b"processedClaim".as_ref(), processor_address.key().as_ref(), processor_count_index.to_le_bytes().as_ref()], 
        bump)]
    pub processed_claim: Box<Account<'info, ProcessedClaim>>,

    #[account(
        seeds = [b"m4aProtocolTreasurer".as_ref()],
        bump)]
    pub treasurer: Account<'info, M4AProtocolTreasurer>,

    #[account(
        mut,
        associated_token::mint = fee_token_entry.token_mint_address,
        associated_token::authority = signer
    )]
    pub user_fee_ata: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = fee_token_entry.token_mint_address,
        associated_token::authority = treasurer.address
        //address = anchor_spl::associated_token::get_associated_token_address(&treasurer.address, &USDC_MINT)
    )]
    pub treasurer_usdc_ata: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"feeTokenEntry".as_ref(),
        token_mint_address.key().as_ref()], 
        bump)]
    pub fee_token_entry: Account<'info, FeeTokenEntry>,

    pub token_program: Program<'info, Token>,
    
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(processor_address: Pubkey, processor_count_index: u64)]
pub struct DenyAppealedClaimWithAllRecords<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        mut,
        seeds = [b"submitter".as_ref(), processed_claim.submitter_address.key().as_ref()],
        bump)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(
        mut, 
        seeds = [b"state".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref()],
        bump)]
    pub state: Account<'info, StateAccount>,

    #[account(
        mut, 
        seeds = [b"patientRecord".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref(), processed_claim.patient_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub patient_record: Account<'info, PatientRecord>,  

    #[account(
        mut, 
        seeds = [b"hospital".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref(), processed_claim.hospital_index.to_le_bytes().as_ref()],
        bump)]
    pub hospital: Account<'info, Hospital>,

    #[account(
        mut, 
        seeds = [b"hospitalRecord".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref(), processed_claim.hospital_index.to_le_bytes().as_ref(), processed_claim.hospital_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub hospital_record: Account<'info, HospitalRecord>,  

    #[account(
        mut,
        seeds = [b"insuranceCompany".as_ref(), processed_claim.insurance_company_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company: Account<'info, InsuranceCompany>,  

    #[account(
        mut, 
        seeds = [b"insuranceCompanyRecord".as_ref(), processed_claim.insurance_company_index.to_le_bytes().as_ref(), processed_claim.insurance_company_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company_record: Account<'info, InsuranceCompanyRecord>,

    #[account(
        mut, 
        seeds = [b"processedClaim".as_ref(), processor_address.key().as_ref(), processor_count_index.to_le_bytes().as_ref()], 
        bump)]
    pub processed_claim: Box<Account<'info, ProcessedClaim>>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(processor_address: Pubkey, processor_count_index: u64)]
pub struct UndenyClaimAndCreateHospitalAndInsuranceCompanyRecords<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Box<Account<'info, M4AProtocolCEO>>,

    #[account(
        mut,
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Box<Account<'info, ProcessorStats>>,

    #[account(
        mut, 
        seeds = [b"submitter".as_ref(), processed_claim.submitter_address.key().as_ref()],
        bump)]
    pub submitter: Box<Account<'info, SubmitterAccount>>,

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut, 
        seeds = [b"state".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref()],
        bump)]
    pub state: Box<Account<'info, StateAccount>>,

    #[account(
        mut, 
        seeds = [b"patientRecord".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref(), processed_claim.patient_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub patient_record: Box<Account<'info, PatientRecord>>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Box<Account<'info, ProcessorAccount>>,

    #[account(
        mut, 
        seeds = [b"hospital".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref(), processed_claim.hospital_index.to_le_bytes().as_ref()],
        bump)]
    pub hospital: Box<Account<'info, Hospital>>,

    #[account(
        init, 
        payer = signer,
        seeds = [b"hospitalRecord".as_ref(),
        processed_claim.country_index.to_le_bytes().as_ref(),
        processed_claim.state_index.to_le_bytes().as_ref(),
        processed_claim.hospital_index.to_le_bytes().as_ref(),
        hospital.record_count.to_le_bytes().as_ref()], 
        bump, 
        space = size_of::<HospitalRecord>() + HOSPITAL_RECORD_EXTRA_SIZE + 8)]
    pub hospital_record: Account<'info, HospitalRecord>, 

    #[account(
        mut,
        seeds = [b"insuranceCompany".as_ref(), processed_claim.insurance_company_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company: Box<Account<'info, InsuranceCompany>>,  

    #[account(
        init, 
        payer = signer,
        seeds = [b"insuranceCompanyRecord".as_ref(), processed_claim.insurance_company_index.to_le_bytes().as_ref(), insurance_company.record_count.to_le_bytes().as_ref()], 
        bump, 
        space = size_of::<InsuranceCompanyRecord>() + INSURANCE_COMPANY_RECORD_EXTRA_SIZE + 8)]
    pub insurance_company_record: Account<'info, InsuranceCompanyRecord>,

    #[account(
        mut, 
        seeds = [b"processedClaim".as_ref(), processor_address.key().as_ref(), processor_count_index.to_le_bytes().as_ref()], 
        bump)]
    pub processed_claim: Box<Account<'info, ProcessedClaim>>,  

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(processor_address: Pubkey, processor_count_index: u64)]
pub struct UndenyClaimWithAllRecords<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut,
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Box<Account<'info, ProcessorStats>>,

    #[account(
        mut,
        seeds = [b"submitter".as_ref(), processed_claim.submitter_address.key().as_ref()],
        bump)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(
        mut, 
        seeds = [b"state".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref()],
        bump)]
    pub state: Account<'info, StateAccount>,

    #[account(
        mut, 
        seeds = [b"patientRecord".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref(), processed_claim.patient_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub patient_record: Account<'info, PatientRecord>,  

    #[account(
        mut, 
        seeds = [b"hospital".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref(), processed_claim.hospital_index.to_le_bytes().as_ref()],
        bump)]
    pub hospital: Account<'info, Hospital>,

    #[account(
        mut, 
        seeds = [b"hospitalRecord".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref(), processed_claim.hospital_index.to_le_bytes().as_ref(), processed_claim.hospital_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub hospital_record: Account<'info, HospitalRecord>,  

    #[account(
        mut,
        seeds = [b"insuranceCompany".as_ref(), processed_claim.insurance_company_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company: Account<'info, InsuranceCompany>,  

    #[account(
        mut, 
        seeds = [b"insuranceCompanyRecord".as_ref(), processed_claim.insurance_company_index.to_le_bytes().as_ref(), processed_claim.insurance_company_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company_record: Account<'info, InsuranceCompanyRecord>,

    #[account(
        mut, 
        seeds = [b"processedClaim".as_ref(), processor_address.key().as_ref(), processor_count_index.to_le_bytes().as_ref()], 
        bump)]
    pub processed_claim: Box<Account<'info, ProcessedClaim>>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(processor_address: Pubkey, processor_count_index: u64, hospital_index: u32, insurance_company_index: u16)]
pub struct EditProcessedClaimAndPatientRecord<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut, 
        seeds = [b"processedClaim".as_ref(), processor_address.key().as_ref(), processor_count_index.to_le_bytes().as_ref()], 
        bump)]
    pub processed_claim: Account<'info, ProcessedClaim>,

    #[account(
        mut, 
        seeds = [b"patientRecord".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref(), processed_claim.patient_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub patient_record: Account<'info, PatientRecord>,

    #[account(
        mut, 
        seeds = [b"hospital".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref(), hospital_index.to_le_bytes().as_ref()],
        bump)]
    pub hospital: Account<'info, Hospital>,

    #[account(
        mut,
        seeds = [b"insuranceCompany".as_ref(), insurance_company_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company: Account<'info, InsuranceCompany>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(processor_address: Pubkey, processor_count_index: u64)]
pub struct EditProcessedClaimAndAllRecords<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        mut, 
        seeds = [b"processedClaim".as_ref(), processor_address.key().as_ref(), processor_count_index.to_le_bytes().as_ref()], 
        bump)]
    pub processed_claim: Box<Account<'info, ProcessedClaim>>,  

    #[account(
        mut, 
        seeds = [b"submitter".as_ref(), processed_claim.submitter_address.key().as_ref()],
        bump)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(
        mut, 
        seeds = [b"state".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref()],
        bump)]
    pub state: Account<'info, StateAccount>,

    #[account(
        mut, 
        seeds = [b"patientRecord".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref(), processed_claim.patient_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub patient_record: Account<'info, PatientRecord>,  

    #[account(
        mut, 
        seeds = [b"hospital".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref(), processed_claim.hospital_index.to_le_bytes().as_ref()],
        bump)]
    pub hospital: Account<'info, Hospital>,

    #[account(
        mut, 
        seeds = [b"hospitalRecord".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref(), processed_claim.hospital_index.to_le_bytes().as_ref(), processed_claim.hospital_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub hospital_record: Account<'info, HospitalRecord>,  

    #[account(
        mut,
        seeds = [b"insuranceCompany".as_ref(), processed_claim.insurance_company_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company: Account<'info, InsuranceCompany>,  

    #[account(
        mut, 
        seeds = [b"insuranceCompanyRecord".as_ref(), processed_claim.insurance_company_index.to_le_bytes().as_ref(), processed_claim.insurance_company_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company_record: Account<'info, InsuranceCompanyRecord>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
#[instruction(processor_address: Pubkey, processor_count_index: u64)]
pub struct RevokeApproval<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Box<Account<'info, ProcessorStats>>,

    #[account(
        mut,
        seeds = [b"submitter".as_ref(), processed_claim.submitter_address.key().as_ref()],
        bump)]
    pub submitter: Account<'info, SubmitterAccount>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(
        mut, 
        seeds = [b"patient".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref()],
        bump)]
    pub patient: Account<'info, PatientAccount>,

    #[account(
        mut, 
        seeds = [b"state".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref()],
        bump)]
    pub state: Account<'info, StateAccount>,

    #[account(
        mut, 
        seeds = [b"patientRecord".as_ref(), processed_claim.submitter_address.key().as_ref(), processed_claim.patient_index.to_le_bytes().as_ref(), processed_claim.patient_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub patient_record: Account<'info, PatientRecord>,

    #[account(
        mut, 
        seeds = [b"hospital".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref(), processed_claim.hospital_index.to_le_bytes().as_ref()],
        bump)]
    pub hospital: Account<'info, Hospital>,

    #[account(
        mut, 
        seeds = [b"hospitalRecord".as_ref(), processed_claim.country_index.to_le_bytes().as_ref(), processed_claim.state_index.to_le_bytes().as_ref(), processed_claim.hospital_index.to_le_bytes().as_ref(), processed_claim.hospital_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub hospital_record: Account<'info, HospitalRecord>,  

    #[account(
        mut,
        seeds = [b"insuranceCompany".as_ref(), processed_claim.insurance_company_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company: Box<Account<'info, InsuranceCompany>>,  

    #[account(
        mut, 
        seeds = [b"insuranceCompanyRecord".as_ref(), processed_claim.insurance_company_index.to_le_bytes().as_ref(), processed_claim.insurance_company_record_index.to_le_bytes().as_ref()], 
        bump)]
    pub insurance_company_record: Account<'info, InsuranceCompanyRecord>,

    #[account(
        mut, 
        seeds = [b"processedClaim".as_ref(), processor_address.key().as_ref(), processor_count_index.to_le_bytes().as_ref()], 
        bump)]
    pub processed_claim: Box<Account<'info, ProcessedClaim>>,  

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct DropDenialHammer<'info> 
{
    #[account(
        seeds = [b"m4aProtocolCEO".as_ref()],
        bump)]
    pub ceo: Account<'info, M4AProtocolCEO>,

    #[account(
        mut, 
        seeds = [b"processorStats".as_ref()],
        bump)]
    pub processor_stats: Account<'info, ProcessorStats>,

    #[account(
        mut,
        seeds = [b"claimQueue".as_ref()],
        bump)]
    pub claim_queue: Account<'info, ClaimQueue>,

    #[account(
        mut, 
        seeds = [b"processor".as_ref(), signer.key().as_ref()],
        bump)]
    pub processor: Account<'info, ProcessorAccount>,

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>
}

//Accounts
#[account]
pub struct M4AProtocolCEO
{
    pub address: Pubkey
}

#[account]
pub struct M4AProtocolTreasurer
{
    pub address: Pubkey
}

#[account]
pub struct FeeTokenEntry
{
    pub token_mint_address: Pubkey,
    pub decimal_amount: u8
}

#[account]
pub struct M4AProtocol
{
    pub m4a_protocol_initiator_address: Pubkey,
    pub submitter_account_total: u64,
    pub patient_account_total: u64,
    pub state_account_total: u32
}

#[account]
pub struct ClaimQueue
{   pub submitted_claim_count: u64,
    pub current_claim_queue_count: u32,
    pub queue_size_limit: u32,
    pub enabled: bool
}

#[account]
pub struct ProcessorStats
{
    pub processor_account_total: u64,
    pub processor_active_account_total: u64,
    pub processor_super_admin_account_total: u64,
    pub set_or_unset_processor_on_claim_count: u64,  //Helps listners to update tables
    pub edited_processor_count: u32,
    pub created_patient_record_count: u64,
    pub created_hospital_and_insurance_company_records_count: u64,
    pub processed_claim_count: u64,
    pub edited_claim_or_processed_claim_count: u64,
    pub approved_claim_amount: u64,
    pub approved_claim_count: u64,
    pub max_denied_claim_count: u64,
    pub denied_claim_count: u64,
    pub undenied_claim_count: u64,
    pub submitted_appeal_count: u64,
    pub denied_appeal_count: u64,
    pub revoked_approval_count: u64,
    pub denial_hammer_dropped_count: u64
}

#[account]
pub struct HospitalStats
{
    pub hospital_count: u32,
    pub general_hospital_count: u32,
    pub dental_hospital_count: u32,
    pub vision_hospital_count: u32,
    pub mental_hospital_count: u32,
    pub edited_hospital_count: u32
}

#[account]
pub struct InsuranceCompanyStats
{
    pub initialized_insurance_company_count: u16,
    pub additional_insurance_company_count: u16,
    pub edited_insurance_company_count: u32
}

#[account]
pub struct SubmitterAccount
{
    pub id: u64,
    pub address: Pubkey,
    pub active_patient_count: u8,
    pub patient_count: u8,
    pub submitted_claim_count: u32,
    pub approved_claim_amount: u64,
    pub approved_claim_count: u32,
    pub max_denied_claim_count: u32,
    pub denied_claim_count: u32,
    pub undenied_claim_count: u32,
    pub submitted_appeal_count: u32,
    pub denied_appeal_count: u32,
    pub revoked_approval_count: u32
}

#[account]
pub struct PatientAccount
{
    pub id: u64,
    pub submitter_address: Pubkey,
    pub is_active: bool,
    pub patient_first_name: String,
    pub patient_last_name: String,
    pub record_count: u32,
    pub edited_record_count: u32, //Helps listners to update records
    pub submitted_claim_count: u32,
    pub approved_claim_amount: u64,
    pub approved_claim_count: u32,
    pub max_denied_claim_count: u32,
    pub denied_claim_count: u32,
    pub undenied_claim_count: u32,
    pub submitted_appeal_count: u32,
    pub denied_appeal_count: u32,
    pub revoked_approval_count: u32 
}

#[account]
pub struct ProcessorAccount
{
    pub id: u64,
    pub address: Pubkey,
    pub is_active: bool,
    pub is_super_admin: bool,
    pub is_processing_claim: bool,
    pub submitter_address_of_claim_being_processed: Pubkey,
    pub created_patient_record_count: u64,
    pub created_hospital_count: u64,
    pub created_hospital_record_count: u64,
    pub created_insurance_company_count: u16,
    pub created_insurance_company_record_count: u32,
    pub processed_claim_count: u64,
    pub approved_claim_amount: u64,
    pub approved_claim_count: u64,
    pub max_denied_claim_count: u64,
    pub denied_claim_count: u64,
    pub undenied_claim_count: u64,
    pub denied_appeal_count: u64,
    pub revoked_approval_count: u64,
    pub denial_hammer_dropped_count: u64
}    

#[account]
pub struct Claim
{
    pub id: u64,
    pub status: u8,
    pub is_patient_record_created: bool,
    pub is_hospital_record_created: bool,
    pub is_insurance_company_record_created: bool,
    pub patient_record_index: u32,
    pub hospital_record_index: u64,
    pub insurance_company_record_index: u64,
    pub submitter_address: Pubkey,
    pub processor_address: Pubkey,
    pub patient_index: u8,
    pub country_index: u16,
    pub state_index: u32,
    pub hospital_index: i32,
    pub hospital_type: u8,
    pub hospital_name: String,
    pub hospital_address: String,
    pub hospital_city: String,
    pub hospital_zip_code: u32,
    pub hospital_phone_number: u128,
    pub hospital_bill_invoice_number: String,
    pub note: String,
    pub claim_amount: u64,
    pub ailment: String,
    pub submitted_time: u64,
    pub insurance_company_index: i16,
    pub insurance_company_name: String
}

#[account]
pub struct ProcessedClaim
{
    pub processed_claim_id: u64,
    pub claim_id: u64,
    pub processor_count_index: u64,
    pub status: u8,
    pub denial_reason: String,
    pub appeal_reason: String,
    pub is_patient_record_created: bool,
    pub is_hospital_record_created: bool,
    pub is_insurance_company_record_created: bool,
    pub patient_record_index: u32,
    pub hospital_record_index: u64,
    pub insurance_company_record_index: u64,
    pub processor_address: Pubkey,
    pub submitter_address: Pubkey,
    pub patient_index: u8,
    pub country_index: u16,
    pub state_index: u32,
    pub hospital_index: i32,
    pub hospital_type: u8,
    pub hospital_name: String,
    pub hospital_address: String,
    pub hospital_city: String,
    pub hospital_zip_code: u32,
    pub hospital_phone_number: u128,
    pub hospital_bill_invoice_number: String,
    pub note: String,
    pub claim_amount: u64,
    pub ailment: String,
    pub submitted_time: u64,
    pub processed_time: u64,
    pub insurance_company_index: i16,
    pub insurance_company_name: String
}

#[account]
pub struct StateAccount
{
    pub id: u32,
    pub index: u32,
    pub approved_claim_amount: u64,
    pub approved_claim_count: u64,
    pub denied_claim_count: u64,
    pub undenied_claim_count: u64,
    pub submitted_appeal_count: u64,
    pub denied_appeal_count: u64,
    pub revoked_approval_count: u64,
    pub hospital_count: u32,
    pub general_hospital_count: u32,
    pub dental_hospital_count: u32,
    pub vision_hospital_count: u32,
    pub mental_hospital_count: u32,
    pub edited_hospital_count: u32
}

#[account]
pub struct PatientRecord
{
    pub record_id: u32,
    pub claim_id: u32,
    pub status: u8,
    pub patient_record_only: bool,
    pub denial_reason: String,
    pub appeal_reason: String,
    pub submitter_address: Pubkey,
    pub processor_address: Pubkey,
    pub processor_count_index: u64,
    pub country_index: u16,
    pub state_index: u32,
    pub hospital_index: u32,
    pub insurance_company_index: u16,
    pub hospital_bill_invoice_number: String,
    pub claim_amount: u64,
    pub ailment: String,
    pub note: String,
    pub submitted_time: u64,
    pub processed_time: u64
}

#[account]
pub struct Hospital
{
    pub id: u32,
    pub is_active: bool,
    pub country_index: u16,
    pub state_index: u32,
    pub hospital_index: u32,
    pub hospital_type: u8,
    pub hospital_longitude: f64,
    pub hospital_latitude: f64,
    pub hospital_name: String,
    pub hospital_address: String,
    pub hospital_city: String,
    pub hospital_zip_code: u32,
    pub hospital_phone_number: u128,
    pub note: String,
    pub record_count: u64,
    pub edited_record_count: u32, //Helps listners to update records
    pub approved_claim_amount: u64,
    pub approved_claim_count: u64,
    pub denied_claim_count: u64,
    pub undenied_claim_count: u64,
    pub submitted_appeal_count: u64,
    pub denied_appeal_count: u64,
    pub revoked_approval_count: u64,
}

#[account]
pub struct HospitalRecord
{
    pub record_id: u64,
    pub claim_id: u64,
    pub status: u8,
    pub denial_reason: String,
    pub appeal_reason: String,
    pub submitter_address: Pubkey,
    pub patient_index: u8,
    pub processor_address: Pubkey,
    pub processor_count_index: u64,
    pub country_index: u16,
    pub state_index: u32,
    pub insurance_company_index: u16,
    pub hospital_bill_invoice_number: String,
    pub claim_amount: u64,
    pub ailment: String,
    pub note: String,
    pub submitted_time: u64,
    pub processed_time: u64,
}

#[account]
pub struct InsuranceCompany
{
    pub id: u16,
    pub insurance_company_index: u16,
    pub is_active: bool,
    pub insurance_company_name: String,
    pub note: String,
    pub record_count: u64,
    pub edited_record_count: u32, //Helps listners to update records
    pub approved_claim_amount: u64,
    pub approved_claim_count: u64,
    pub denied_claim_count: u64,
    pub undenied_claim_count: u64,
    pub submitted_appeal_count: u64,
    pub denied_appeal_count: u64,
    pub revoked_approval_count: u64, 
}

#[account]
pub struct InsuranceCompanyRecord
{
    pub record_id: u64,
    pub claim_id: u64,
    pub status: u8,
    pub denial_reason: String,
    pub appeal_reason: String,
    pub submitter_address: Pubkey,
    pub patient_index: u8,
    pub processor_address: Pubkey,
    pub processor_count_index: u64,
    pub country_index: u16,
    pub state_index: u32,
    pub hospital_index: u32,
    pub hospital_bill_invoice_number: String,
    pub claim_amount: u64,
    pub ailment: String,
    pub note: String,
    pub submitted_time: u64,
    pub processed_time: u64
}