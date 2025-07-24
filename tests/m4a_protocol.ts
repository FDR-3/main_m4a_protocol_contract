import * as anchor from "@coral-xyz/anchor"
import { Program } from "@coral-xyz/anchor"
import { M4AProtocol } from "../target/types/m_4_a_protocol"
import { utf8 } from "@coral-xyz/anchor/dist/cjs/utils/bytes"
import { assert } from "chai"

describe("M4A_Protocol", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env())

  const program = anchor.workspace.M4AProtocol as Program<M4AProtocol>

  //default signing wallet string
  //program.provider.publicKey.toBase58()

  const patientFirstName = "Lorem ipsum dolor sit amet, consectetuer adipiscing."
  const patientLastName = "Lorem ipsum dolor sit amet, consectetuer adipiscing."
  const patientIndex = 0
  const countryIndex = 0
  const stateIndex = 0
  const hospitalIndex = 0
  const hospitalType= 0
  const hospitalLongitude= -86.719172
  const hospitalLatitude = 32.650996
  const hospitalName = "Lorem ipsum dolor sit amet, consectetuer adipiscin"
  const hospitalAddress = "Lorem ipsum dolor sit amet, consectetuer adipiscing elit. Aenean commodo ligula eget dolor. Aenean m"
  const hospitalCity= "Lorem ipsum dolor sit amet, consectetuer"
  const hospitalZipCode = 77777
  const hospitalPhoneNumber = new anchor.BN(9007199254740991)//3.4028236692093846346337460743177e+38//    
  const hospitalBillInvoiceNumber = "Lorem ipsum dolor si"  
  const note144Characters = "Lorem ipsum dolor sit amet, consectetuer adipiscing elit. Aenean commodo ligula eget dolor. Aenean massa. Cum sociis natoque penatibus et magnis"
  const claimAmount = new anchor.BN(10000)
  const ailment = "Lorem ipsum dolor sit amet, consectetuer adip"
  const insuranceCompanyIndex = 0
  const insuranceCompanyName = "😂LMAO I don't have insurance😂"

  let firstCustomerWallet = anchor.web3.Keypair.generate()

  it("Initializes M4A Protocol CEO Account", async () => 
  {
    await program.methods.initializeM4AProtocolCeoAccount().rpc()

    var ceoAccount = await program.account.m4AProtocolCeo.fetch(getM4AProtocolCEOAccountPDA())
    assert(ceoAccount.address.toBase58() == program.provider.publicKey.toBase58())
  })

  it("Initializes Protocol Stats", async () => 
  {
    await program.methods.initializeProtocolStats().rpc()
  })
  
  it("Initializes M4A Protocol And Claim Queue", async () => 
  {
    let token_airdrop = await program.provider.connection.requestAirdrop(firstCustomerWallet.publicKey, 
    10 * 1000000000) //1 billion lamports equals 1 SOL

    const latestBlockHash = await program.provider.connection.getLatestBlockhash()
    await program.provider.connection.confirmTransaction
    ({
      blockhash: latestBlockHash.blockhash,
      lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
      signature: token_airdrop
    })

    await program.methods.initializeM4AProtocolAndClaimQueue()
    .accounts({signer: firstCustomerWallet.publicKey})
    .signers([firstCustomerWallet])
    .rpc()
  })

  it("Passes on the M4A Protocol CEO Account", async () => 
  {
    await program.methods.passOnM4AProtocolCeo(firstCustomerWallet.publicKey).rpc()
    
    var ceoAccount = await program.account.m4AProtocolCeo.fetch(getM4AProtocolCEOAccountPDA())
    assert(ceoAccount.address.toBase58() == firstCustomerWallet.publicKey.toBase58())
  })

  it("Passes back the M4A Protocol CEO Account", async () => 
  {
    await program.methods.passOnM4AProtocolCeo(program.provider.publicKey).
    accounts({signer: firstCustomerWallet.publicKey})
    .signers([firstCustomerWallet])
    .rpc()
    
    var ceoAccount = await program.account.m4AProtocolCeo.fetch(getM4AProtocolCEOAccountPDA())
    assert(ceoAccount.address.toBase58() == program.provider.publicKey.toBase58())
  })

  it("Disables the Claim Que", async () => 
  {
    await program.methods.setClaimQueueFlag(false).rpc()
    
    var claimQueue = await program.account.claimQueue.fetch(getClaimQueuePDA())
    assert(claimQueue.enabled == false)
  }) 

  it("Enables the Claim Que", async () => 
  {
    await program.methods.setClaimQueueFlag(true).rpc()
    
    var claimQueue = await program.account.claimQueue.fetch(getClaimQueuePDA())
    assert(claimQueue.enabled == true)
  }) 

  it("Creates Submitter Account", async () => 
  {
    await program.methods.createSubmitterAccount()
    .accounts({signer: firstCustomerWallet.publicKey})
    .signers([firstCustomerWallet])
    .rpc()
  })

  it("Creates Patient Account", async () => 
  {
    await program.methods.createPatientAccount(patientFirstName, patientLastName)
    .accounts({signer: firstCustomerWallet.publicKey})
    .signers([firstCustomerWallet])
    .rpc()

    var patient = await program.account.patientAccount.fetch(getPatientPDA(firstCustomerWallet.publicKey, patientIndex))

    assert(patient.patientFirstName == patientFirstName)
    assert(patient.patientLastName == patientLastName)
  })

  it("Creates Processor Account", async () => 
  {
    await program.methods.createProcessorAccount(program.provider.publicKey).rpc()
    var processor = await program.account.processorAccount.fetch(getProcessorPDA(program.provider.publicKey))

    assert(processor.isActive == true)
    assert(processor.isSuperAdmin == false)
  })

  it("Sets Processor Account As Inactive", async () => 
  {
    await program.methods.setProcessorAccountActiveFlag(program.provider.publicKey, false).rpc()
    var processor = await program.account.processorAccount.fetch(getProcessorPDA(program.provider.publicKey))

    assert(processor.isActive == false)
  })

  it("Sets Processor Account As Active", async () => 
  {
    await program.methods.setProcessorAccountActiveFlag(program.provider.publicKey, true).rpc()
    var processor = await program.account.processorAccount.fetch(getProcessorPDA(program.provider.publicKey))

    assert(processor.isActive == true)
  })

  it("Sets Processor Account As Admin", async () => 
  {
    await program.methods.setProcessorAccountPrivilege(program.provider.publicKey, true).rpc()
    var processor = await program.account.processorAccount.fetch(getProcessorPDA(program.provider.publicKey))

    assert(processor.isSuperAdmin == true)
  })

  it("Unsets Processor Account As Admin", async () => 
  {
    await program.methods.setProcessorAccountPrivilege(program.provider.publicKey, false).rpc()
    var processor = await program.account.processorAccount.fetch(getProcessorPDA(program.provider.publicKey))

    assert(processor.isSuperAdmin == false)
  })

  it("Submits A Claim To The Queue", async () => 
  {
    await program.methods.submitClaimToQueue
    (
      patientIndex,
      countryIndex,
      stateIndex,
      hospitalIndex,
      hospitalType,
      hospitalName,
      hospitalAddress,
      hospitalCity,
      hospitalZipCode,
      hospitalPhoneNumber,
      hospitalBillInvoiceNumber,
      note144Characters,
      claimAmount,
      ailment,
      insuranceCompanyIndex,
      insuranceCompanyName)
    .accounts({signer: firstCustomerWallet.publicKey})
    .signers([firstCustomerWallet])
    .rpc()
  })

  it("Marks Claim For Processing", async () => 
  {
    await program.methods.assignClaimToProcessor(firstCustomerWallet.publicKey).rpc()
  })
  
  it("Creates State Account", async () => 
  {
    await program.methods.createStateAccount(firstCustomerWallet.publicKey, countryIndex, stateIndex).rpc()
  })

  it("Creates Hospital", async () => 
    {
      await program.methods.createHospital
      (
        firstCustomerWallet.publicKey,
        countryIndex, 
        stateIndex, 
        hospitalType,
        hospitalLongitude,
        hospitalLatitude,
        hospitalName, 
        hospitalAddress,
        hospitalCity,
        hospitalZipCode,
        hospitalPhoneNumber,
        note144Characters).rpc()
    })

  it("Creates Insurance Company", async () => 
  {
    await program.methods.createInsuranceCompany(firstCustomerWallet.publicKey, insuranceCompanyIndex, insuranceCompanyName, note144Characters).rpc()
  })

  it("Creates Patient Record", async () => 
  {
    await program.methods.createPatientRecord(firstCustomerWallet.publicKey).rpc()
  })

  it("Creates Hospital And Insurance Company Records", async () => 
  {
    await program.methods.createHospitalAndInsuranceCompanyRecords(firstCustomerWallet.publicKey).rpc()
  })

  it("Approves Claim", async () => 
  {
    var processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
    console.log("Processed Claim Count: ", processorStats.processedClaimCount)
    console.log("Approved Claim Count: ", processorStats.approvedClaimCount)

    await program.methods.approveClaim(firstCustomerWallet.publicKey).rpc()

    processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
    console.log("Processed Claim Count: ", processorStats.processedClaimCount)
    console.log("Approved Claim Count: ", processorStats.approvedClaimCount)
  })

  it("Submits and Max denies pending claims", async () => 
  {
    //Submit 100 Claims
    for(var i=1; i<=1; i++)
    {
      //Fund Wallet
      let newWallet = anchor.web3.Keypair.generate()
      let token_airdrop = await program.provider.connection.requestAirdrop(newWallet.publicKey, 
        1000 * 10002240)

      const latestBlockHash = await program.provider.connection.getLatestBlockhash()
      await program.provider.connection.confirmTransaction
      ({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: token_airdrop,
      })

      //Init Submitter Account
      await program.methods.createSubmitterAccount()
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      //Init Patient Account
      const patientFirstName = "John"
      const patientLastName = "Doe"
      await program.methods.createPatientAccount(patientFirstName, patientLastName)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.submitClaimToQueue
      (
        patientIndex,
        countryIndex,
        stateIndex,
        hospitalIndex,
        hospitalType,
        hospitalName,
        hospitalAddress,
        hospitalCity,
        hospitalZipCode,
        hospitalPhoneNumber,
        hospitalBillInvoiceNumber,
        note144Characters,
        claimAmount,
        ailment,
        insuranceCompanyIndex,
        insuranceCompanyName
      )
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      var processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("MaxDeniedClaim Count: ", processorStats.maxDeniedClaimCount)

      await program.methods.maxDenyPendingClaim(newWallet.publicKey).rpc()
      var processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())

      //const derp = await program.account.processedClaim.all()
      //console.log("PDA Actual: ", derp[i-1])
      //console.log("PDA Helper: ", getProcessedClaimPDA(i-1))

      console.log("MaxDeniedClaim Count: ", processorStats.maxDeniedClaimCount)
    }
  })

  it("Submits and Max in progress claims", async () => 
    {
      //Submit 100 Claims
      for(var i=1; i<=1; i++)
      {
        //Fund Wallet
        let newWallet = anchor.web3.Keypair.generate()
        let token_airdrop = await program.provider.connection.requestAirdrop(newWallet.publicKey, 
          1000 * 10002240)
  
        const latestBlockHash = await program.provider.connection.getLatestBlockhash()
        await program.provider.connection.confirmTransaction
        ({
          blockhash: latestBlockHash.blockhash,
          lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
          signature: token_airdrop,
        })
  
        //Init Submitter Account
        await program.methods.createSubmitterAccount()
        .accounts({signer: newWallet.publicKey})
        .signers([newWallet])
        .rpc()
  
        //Init Patient Account
        const patientFirstName = "John"
        const patientLastName = "Doe"
        await program.methods.createPatientAccount(patientFirstName, patientLastName)
        .accounts({signer: newWallet.publicKey})
        .signers([newWallet])
        .rpc()
  
        await program.methods.submitClaimToQueue
        (
          patientIndex,
          countryIndex,
          stateIndex,
          hospitalIndex,
          hospitalType,
          hospitalName,
          hospitalAddress,
          hospitalCity,
          hospitalZipCode,
          hospitalPhoneNumber,
          hospitalBillInvoiceNumber,
          note144Characters,
          claimAmount,
          ailment,
          insuranceCompanyIndex,
          insuranceCompanyName
        )
        .accounts({signer: newWallet.publicKey})
        .signers([newWallet])
        .rpc()

        await program.methods.assignClaimToProcessor(newWallet.publicKey).rpc()
  
        var processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
        
        console.log("Processed Claim Count: ", processorStats.processedClaimCount)
        console.log("MaxDeniedClaim Count: ", processorStats.maxDeniedClaimCount)
  
        await program.methods.maxDenyInProgressClaim(newWallet.publicKey).rpc()
        var processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
  
        //const derp = await program.account.processedClaim.all()
        //console.log("PDA Actual: ", derp[i-1])
        //console.log("PDA Helper: ", getProcessedClaimPDA(i-1))
  
        console.log("MaxDeniedClaim Count: ", processorStats.maxDeniedClaimCount)
      }
    })

  it("Submits and denies claims", async () => 
  {
    //Submit 100 Claims
    for(var i=1; i<=1; i++)
    {
      //Fund Wallet
      let newWallet = anchor.web3.Keypair.generate()
      let token_airdrop = await program.provider.connection.requestAirdrop(newWallet.publicKey, 
        1000 * 10002240)

      const latestBlockHash = await program.provider.connection.getLatestBlockhash()
      await program.provider.connection.confirmTransaction
      ({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: token_airdrop,
      })

      //Init Submitter Account
      await program.methods.createSubmitterAccount()
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      //Init Patient Account
      const patientFirstName = "John"
      const patientLastName = "Doe"
      await program.methods.createPatientAccount(patientFirstName, patientLastName)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.submitClaimToQueue
      (
        patientIndex,
        countryIndex,
        stateIndex,
        hospitalIndex,
        hospitalType,
        hospitalName,
        hospitalAddress,
        hospitalCity,
        hospitalZipCode,
        hospitalPhoneNumber,
        hospitalBillInvoiceNumber,
        note144Characters,
        claimAmount,
        ailment,
        insuranceCompanyIndex,
        insuranceCompanyName
      )
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.assignClaimToProcessor(newWallet.publicKey).rpc()
      
      await program.methods.createPatientRecord(newWallet.publicKey).rpc()
      await program.methods.createHospitalAndInsuranceCompanyRecords(newWallet.publicKey).rpc()
    
      var processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())

      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("DeniedClaim Count: ", processorStats.deniedClaimCount)

      const denialReason = "Testing"
      await program.methods.denyClaimWithAllRecords(newWallet.publicKey, denialReason).rpc()
      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())

      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("DeniedClaim Count: ", processorStats.deniedClaimCount)
    }
  })

  it("Submits claims and creates patient record with denial", async () => 
  {
    //Submit 100 Claims
    for(var i=1; i<=1; i++)
    {
      //Fund Wallet
      let newWallet = anchor.web3.Keypair.generate()
      let token_airdrop = await program.provider.connection.requestAirdrop(newWallet.publicKey, 
        1000 * 10002240)

      const latestBlockHash = await program.provider.connection.getLatestBlockhash()
      await program.provider.connection.confirmTransaction
      ({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: token_airdrop,
      })

      //Init Submitter Account
      await program.methods.createSubmitterAccount()
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      //Init Patient Account
      const patientFirstName = "John"
      const patientLastName = "Doe"
      await program.methods.createPatientAccount(patientFirstName, patientLastName)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.submitClaimToQueue
      (
        patientIndex,
        countryIndex,
        stateIndex,
        hospitalIndex,
        hospitalType,
        hospitalName,
        hospitalAddress,
        hospitalCity,
        hospitalZipCode,
        hospitalPhoneNumber,
        hospitalBillInvoiceNumber,
        note144Characters,
        claimAmount,
        ailment,
        insuranceCompanyIndex,
        insuranceCompanyName
      )
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.assignClaimToProcessor(newWallet.publicKey).rpc()

      const denialReason = "Testing"

      var processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("DeniedClaim Count: ", processorStats.deniedClaimCount)

      await program.methods.createPatientRecordAndDenyClaim(newWallet.publicKey, denialReason).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("DeniedClaim Count: ", processorStats.deniedClaimCount)
    }
  })

  it("Submits Claim, Updates Hospital And Insurance Company Indexes, And Approves Claim", async () => 
  {
    //Submit 100 Claims
    for(var i=1; i<=1; i++)
    {
      //Fund Wallet
      let newWallet = anchor.web3.Keypair.generate()
      let token_airdrop = await program.provider.connection.requestAirdrop(newWallet.publicKey, 
        1000 * 10002240)

      const latestBlockHash = await program.provider.connection.getLatestBlockhash()
      await program.provider.connection.confirmTransaction
      ({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: token_airdrop,
      })

      //Init Submitter Account
      await program.methods.createSubmitterAccount()
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      //Init Patient Account
      const patientFirstName = "John"
      const patientLastName = "Doe"
      await program.methods.createPatientAccount(patientFirstName, patientLastName)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      const wrongHospitalIndex = 11
      const wrongInsuranceIndex = 11

      await program.methods.submitClaimToQueue
      (
        patientIndex,
        countryIndex,
        stateIndex,
        wrongHospitalIndex,
        hospitalType,
        hospitalName,
        hospitalAddress,
        hospitalCity,
        hospitalZipCode,
        hospitalPhoneNumber,
        hospitalBillInvoiceNumber,
        note144Characters,
        claimAmount,
        ailment,
        wrongInsuranceIndex,
        insuranceCompanyName
      )
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.assignClaimToProcessor(newWallet.publicKey).rpc()

      await program.methods.updateClaimHospitalIndex(newWallet.publicKey, hospitalIndex).rpc()
      await program.methods.updateClaimInsuranceCompanyIndex(newWallet.publicKey, insuranceCompanyIndex).rpc()

      console.log(`${i} claims updated`)

      await program.methods.createPatientRecord(newWallet.publicKey).rpc()
      await program.methods.createHospitalAndInsuranceCompanyRecords(newWallet.publicKey).rpc()
      await program.methods.approveClaim(newWallet.publicKey).rpc()
    }
  })

  it("Submits Claims To Queue", async () => 
  {
    //Submit 100 Claims
    for(var i=1; i<=1; i++)
    {
      //Fund Wallet
      let newWallet = anchor.web3.Keypair.generate()
      let token_airdrop = await program.provider.connection.requestAirdrop(newWallet.publicKey, 
        1000 * 10002240)

      const latestBlockHash = await program.provider.connection.getLatestBlockhash()
      await program.provider.connection.confirmTransaction
      ({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: token_airdrop,
      })

      //Init Submitter Account
      await program.methods.createSubmitterAccount()
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      //Init Patient Account
      const patientFirstName = "John"
      const patientLastName = "Doe"
      await program.methods.createPatientAccount(patientFirstName, patientLastName)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.submitClaimToQueue
      (
        patientIndex,
        countryIndex,
        stateIndex,
        hospitalIndex,
        hospitalType,
        hospitalName,
        hospitalAddress,
        hospitalCity,
        hospitalZipCode,
        hospitalPhoneNumber,
        hospitalBillInvoiceNumber,
        note144Characters,
        claimAmount,
        ailment,
        insuranceCompanyIndex,
        insuranceCompanyName
      )
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()
    }
  })

  it("Drops Denial Hammer", async () => 
  {
    var claims = await program.account.claim.all()
    
    const chunkSize = 25
    const chunks = chunk(claims, chunkSize)

    for(var i=0; i<chunks.length; i++)
    {
      var claimsToDelete = []

      for(var j=0; j<chunks[i].length; j++)
      {
        var claim = 
        {
          pubkey: claims[j].publicKey,
          isSigner: false,
          isWritable: true
        }

        claimsToDelete.push(claim)
      }
  
      claims = await program.account.claim.all()
      console.log("Claim count before denial hammer: ", claims.length)
  
      await program.methods.dropDenialHammer().accounts({
      }).remainingAccounts(claimsToDelete).rpc()
  
      claims = await program.account.claim.all()
      console.log("Claim count after denial hammer: ", claims.length)
    }
  })

  it("Submits Claims To Queue, Creates All Records, And Approves Claim", async () => 
  {
    //Submit 100 Claims
    for(var i=1; i<=1; i++)
    {
      //Fund Wallet
      let newWallet = anchor.web3.Keypair.generate()
      let token_airdrop = await program.provider.connection.requestAirdrop(newWallet.publicKey, 
        1000 * 10002240)

      const latestBlockHash = await program.provider.connection.getLatestBlockhash()
      await program.provider.connection.confirmTransaction
      ({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: token_airdrop,
      })

      //Init Submitter Account
      await program.methods.createSubmitterAccount()
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      //Init Patient Account
      const patientFirstName = "John"
      const patientLastName = "Doe"
      await program.methods.createPatientAccount(patientFirstName, patientLastName)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.submitClaimToQueue
      (
        patientIndex,
        countryIndex,
        stateIndex,
        hospitalIndex,
        hospitalType,
        hospitalName,
        hospitalAddress,
        hospitalCity,
        hospitalZipCode,
        hospitalPhoneNumber,
        hospitalBillInvoiceNumber,
        note144Characters,
        claimAmount,
        ailment,
        insuranceCompanyIndex,
        insuranceCompanyName
      )
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.assignClaimToProcessor(newWallet.publicKey).rpc()

      var processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)

      await program.methods.createPatientRecord(newWallet.publicKey).rpc()
      await program.methods.createHospitalAndInsuranceCompanyRecords(newWallet.publicKey).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)

      await program.methods.approveClaim(newWallet.publicKey).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)
    }
  })

  it("Approves Claim With Edits", async () => 
  {
    await program.methods.submitClaimToQueue
    (
      patientIndex,
      countryIndex,
      stateIndex,
      hospitalIndex,
      hospitalType,
      hospitalName,
      hospitalAddress,
      hospitalCity,
      hospitalZipCode,
      hospitalPhoneNumber,
      hospitalBillInvoiceNumber,
      note144Characters,
      claimAmount,
      ailment,
      insuranceCompanyIndex,
      insuranceCompanyName)
    .accounts({signer: firstCustomerWallet.publicKey})
    .signers([firstCustomerWallet])
    .rpc()

    await program.methods.assignClaimToProcessor(firstCustomerWallet.publicKey).rpc()

    const hospitalLongitudeEdited = 1.111
    const hospitalLatitudeEdited = 8.88
    const hospitalNameEdited = "Hos Name Edited"
    const hospitalAddressEdited  = "Hos Address Edited"
    const hospitalCityEdited = "Hos City Edited"
    const hospitalZipCodeEdited = 47474
    const hospitalPhoneNumberEdited = new anchor.BN(7777774444)  
    const hospitalBillInvoiceNumberEdited = "#efg"  
    const claimNoteEdited = "Edited Claim Note"
    const claimAmountEdited = new anchor.BN(4712357)
    const ailmentEdited = "Foot Surgery Edited"

    await program.methods.createPatientRecord(firstCustomerWallet.publicKey).rpc()
    await program.methods.createHospitalAndInsuranceCompanyRecords(firstCustomerWallet.publicKey).rpc()

    await program.methods.approveClaimWithEdits
    (
      firstCustomerWallet.publicKey, 
      hospitalType,
      hospitalLongitudeEdited,
      hospitalLatitudeEdited,
      hospitalNameEdited,
      hospitalAddressEdited,
      hospitalCityEdited,
      hospitalZipCodeEdited,
      hospitalPhoneNumberEdited,
      hospitalBillInvoiceNumberEdited,
      claimNoteEdited,
      claimAmountEdited,
      ailmentEdited,
      insuranceCompanyName,
    ).rpc()
  })

  it("Submits Claims To Queue, Creates Patient Record And Denies Claim, Appeals Claim, Creates Hospital And Insurance Company Records, And Then Undenies Claim", async () => 
  {
    //Submit 100 Claims
    for(var i=1; i<=1; i++)
    {
      //Fund Wallet
      let newWallet = anchor.web3.Keypair.generate()
      let token_airdrop = await program.provider.connection.requestAirdrop(newWallet.publicKey, 
        1000 * 10002240)

      const latestBlockHash = await program.provider.connection.getLatestBlockhash()
      await program.provider.connection.confirmTransaction
      ({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: token_airdrop,
      })

      //Init Submitter Account
      await program.methods.createSubmitterAccount()
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      //Init Patient Account
      const patientFirstName = "John"
      const patientLastName = "Doe"
      await program.methods.createPatientAccount(patientFirstName, patientLastName)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.submitClaimToQueue
      (
        patientIndex,
        countryIndex,
        stateIndex,
        hospitalIndex,
        hospitalType,
        hospitalName,
        hospitalAddress,
        hospitalCity,
        hospitalZipCode,
        hospitalPhoneNumber,
        hospitalBillInvoiceNumber,
        note144Characters,
        claimAmount,
        ailment,
        insuranceCompanyIndex,
        insuranceCompanyName
      )
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.assignClaimToProcessor(newWallet.publicKey).rpc()

      var processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)

      const denialReason = "Testing"
      await program.methods.createPatientRecordAndDenyClaim(newWallet.publicKey, denialReason).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)

      const appealReason = "Testing Appeal"
      const processor = await program.account.processorAccount.fetch(getProcessorPDA(program.provider.publicKey))

      await program.methods.appealDeniedClaimWithOnlyPatientRecord(program.provider.publicKey, processor.processedClaimCount.sub(new anchor.BN(1)), appealReason)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)
      console.log("Undenied Claim Count: ", processorStats.undeniedClaimCount)
      
      await program.methods.undenyClaimAndCreateHospitalAndInsuranceCompanyRecords(program.provider.publicKey, processor.processedClaimCount.sub(new anchor.BN(1))).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)
      console.log("Undenied Claim Count: ", processorStats.undeniedClaimCount)
    }
  })

  it("Submits Claims To Queue, Creates All Records, Denies Claim, Appeals Claim, And Then Undenies Claim", async () => 
  {
    //Submit 100 Claims
    for(var i=1; i<=1; i++)
    {
      //Fund Wallet
      let newWallet = anchor.web3.Keypair.generate()
      let token_airdrop = await program.provider.connection.requestAirdrop(newWallet.publicKey, 
        1000 * 10002240)

      const latestBlockHash = await program.provider.connection.getLatestBlockhash()
      await program.provider.connection.confirmTransaction
      ({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: token_airdrop,
      })

      //Init Submitter Account
      await program.methods.createSubmitterAccount()
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      //Init Patient Account
      const patientFirstName = "John"
      const patientLastName = "Doe"
      await program.methods.createPatientAccount(patientFirstName, patientLastName)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.submitClaimToQueue
      (
        patientIndex,
        countryIndex,
        stateIndex,
        hospitalIndex,
        hospitalType,
        hospitalName,
        hospitalAddress,
        hospitalCity,
        hospitalZipCode,
        hospitalPhoneNumber,
        hospitalBillInvoiceNumber,
        note144Characters,
        claimAmount,
        ailment,
        insuranceCompanyIndex,
        insuranceCompanyName
      )
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.assignClaimToProcessor(newWallet.publicKey).rpc()

      var processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)

      await program.methods.createPatientRecord(newWallet.publicKey).rpc()
      await program.methods.createHospitalAndInsuranceCompanyRecords(newWallet.publicKey).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)
      
      const denialReason = "Testing"
      await program.methods.denyClaimWithAllRecords(newWallet.publicKey, denialReason).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)
      
      const appealReason = "Testing Appeal"
      const processor = await program.account.processorAccount.fetch(getProcessorPDA(program.provider.publicKey))

      await program.methods.appealDeniedClaimWithAllRecords(program.provider.publicKey, processor.processedClaimCount.sub(new anchor.BN(1)), appealReason)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)
      console.log("Undenied Claim Count: ", processorStats.undeniedClaimCount)

      await program.methods.undenyClaimWithAllRecords(program.provider.publicKey, processor.processedClaimCount.sub(new anchor.BN(1))).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)
      console.log("Undenied Claim Count: ", processorStats.undeniedClaimCount)
    }
  })

  it("Submits Claims To Queue, Creates Patient Record And Denies Claim, Appeals Claim, And Then Denies Appeal", async () => 
  {
    //Submit 100 Claims
    for(var i=1; i<=1; i++)
    {
      //Fund Wallet
      let newWallet = anchor.web3.Keypair.generate()
      let token_airdrop = await program.provider.connection.requestAirdrop(newWallet.publicKey, 
        1000 * 10002240)

      const latestBlockHash = await program.provider.connection.getLatestBlockhash()
      await program.provider.connection.confirmTransaction
      ({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: token_airdrop,
      })

      //Init Submitter Account
      await program.methods.createSubmitterAccount()
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      //Init Patient Account
      const patientFirstName = "John"
      const patientLastName = "Doe"
      await program.methods.createPatientAccount(patientFirstName, patientLastName)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.submitClaimToQueue
      (
        patientIndex,
        countryIndex,
        stateIndex,
        hospitalIndex,
        hospitalType,
        hospitalName,
        hospitalAddress,
        hospitalCity,
        hospitalZipCode,
        hospitalPhoneNumber,
        hospitalBillInvoiceNumber,
        note144Characters,
        claimAmount,
        ailment,
        insuranceCompanyIndex,
        insuranceCompanyName
      )
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.assignClaimToProcessor(newWallet.publicKey).rpc()

      var processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Denied Claim Count: ", processorStats.deniedClaimCount)

      const denialReason = "Testing"
      await program.methods.createPatientRecordAndDenyClaim(newWallet.publicKey, denialReason).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Denied Claim Count: ", processorStats.deniedClaimCount)

      const appealReason = "Testing Appeal"
      const processor = await program.account.processorAccount.fetch(getProcessorPDA(program.provider.publicKey))

      await program.methods.appealDeniedClaimWithOnlyPatientRecord(program.provider.publicKey, processor.processedClaimCount.sub(new anchor.BN(1)), appealReason)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()
      
      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Denied Appeal Count: ", processorStats.deniedAppealCount)

      const denyAppealReason = "Testing Denying Appeal"
      await program.methods.denyAppealedClaimWithOnlyPatientRecord(program.provider.publicKey, processor.processedClaimCount.sub(new anchor.BN(1)), denyAppealReason).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Denied Appeal Count: ", processorStats.deniedAppealCount)
    }
  })
  
  it("Submits Claims To Queue, Creates All Records, Denies Claim, Appeals Claim, And  Then Denies Appeal", async () => 
  {
    //Submit 100 Claims
    for(var i=1; i<=1; i++)
    {
      //Fund Wallet
      let newWallet = anchor.web3.Keypair.generate()
      let token_airdrop = await program.provider.connection.requestAirdrop(newWallet.publicKey, 
        1000 * 10002240)

      const latestBlockHash = await program.provider.connection.getLatestBlockhash()
      await program.provider.connection.confirmTransaction
      ({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: token_airdrop,
      })

      //Init Submitter Account
      await program.methods.createSubmitterAccount()
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      //Init Patient Account
      const patientFirstName = "John"
      const patientLastName = "Doe"
      await program.methods.createPatientAccount(patientFirstName, patientLastName)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.submitClaimToQueue
      (
        patientIndex,
        countryIndex,
        stateIndex,
        hospitalIndex,
        hospitalType,
        hospitalName,
        hospitalAddress,
        hospitalCity,
        hospitalZipCode,
        hospitalPhoneNumber,
        hospitalBillInvoiceNumber,
        note144Characters,
        claimAmount,
        ailment,
        insuranceCompanyIndex,
        insuranceCompanyName
      )
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.assignClaimToProcessor(newWallet.publicKey).rpc()
      
      await program.methods.createPatientRecord(newWallet.publicKey).rpc()
      await program.methods.createHospitalAndInsuranceCompanyRecords(newWallet.publicKey).rpc()

      var processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Denied Claim Count: ", processorStats.deniedClaimCount)
      
      const denialReason = "Testing"
      await program.methods.denyClaimWithAllRecords(newWallet.publicKey, denialReason).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Denied Claim Count: ", processorStats.deniedClaimCount)
      
      const appealReason = "Testing Appeal"
      const processor = await program.account.processorAccount.fetch(getProcessorPDA(program.provider.publicKey))

      await program.methods.appealDeniedClaimWithAllRecords(program.provider.publicKey, processor.processedClaimCount.sub(new anchor.BN(1)), appealReason)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Denied Appeal Count: ", processorStats.deniedAppealCount)

      const denyAppealReason = "Testing Denying Appeal"
      await program.methods.denyAppealedClaimWithAllRecords(program.provider.publicKey, processor.processedClaimCount.sub(new anchor.BN(1)), denyAppealReason).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Denied Appeal Count: ", processorStats.deniedAppealCount)
    }
  })

  it("Submits Claims To Queue, Creates Patient Record And Denies Claim, And Updates The Processed Claim And Patient Record", async () => 
  {
    //Submit 100 Claims
    for(var i=1; i<=1; i++)
    {
      //Fund Wallet
      let newWallet = anchor.web3.Keypair.generate()
      let token_airdrop = await program.provider.connection.requestAirdrop(newWallet.publicKey, 
        1000 * 10002240)

      const latestBlockHash = await program.provider.connection.getLatestBlockhash()
      await program.provider.connection.confirmTransaction
      ({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: token_airdrop,
      })

      //Init Submitter Account
      await program.methods.createSubmitterAccount()
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      //Init Patient Account
      const patientFirstName = "John"
      const patientLastName = "Doe"
      await program.methods.createPatientAccount(patientFirstName, patientLastName)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.submitClaimToQueue
      (
        patientIndex,
        countryIndex,
        stateIndex,
        hospitalIndex,
        hospitalType,
        hospitalName,
        hospitalAddress,
        hospitalCity,
        hospitalZipCode,
        hospitalPhoneNumber,
        hospitalBillInvoiceNumber,
        note144Characters,
        claimAmount,
        ailment,
        insuranceCompanyIndex,
        insuranceCompanyName
      )
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.assignClaimToProcessor(newWallet.publicKey).rpc()

      var processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)

      const denialReason = "Testing"
      await program.methods.createPatientRecordAndDenyClaim(newWallet.publicKey, denialReason).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)

      const newHospitalBillInvoiceNumber = "abc123"
      const newClaimNote = "NEW PROCESSED CLAIM NOTE" 
      const newClaimAmount = new anchor.BN(77777) //Convert to Fixed Point
      const newAilment = "NEW AILMENT"
 
      const processor = await program.account.processorAccount.fetch(getProcessorPDA(program.provider.publicKey))

      await program.methods.editProcessedClaimAndPatientRecord(
        program.provider.publicKey, 
        processor.processedClaimCount.sub(new anchor.BN(1)),
        hospitalIndex,
        insuranceCompanyIndex,
        newHospitalBillInvoiceNumber,
        newClaimNote,
        newClaimAmount,
        newAilment).rpc()
    }
  })

  it("Submits Claims To Queue, Creates All Records, Approves Claim, And Updates The Processed Claim And All Records", async () => 
  {
    //Submit 100 Claims
    for(var i=1; i<=1; i++)
    {
      //Fund Wallet
      let newWallet = anchor.web3.Keypair.generate()
      let token_airdrop = await program.provider.connection.requestAirdrop(newWallet.publicKey, 
        1000 * 10002240)

      const latestBlockHash = await program.provider.connection.getLatestBlockhash()
      await program.provider.connection.confirmTransaction
      ({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: token_airdrop,
      })

      //Init Submitter Account
      await program.methods.createSubmitterAccount()
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      //Init Patient Account
      const patientFirstName = "John"
      const patientLastName = "Doe"
      await program.methods.createPatientAccount(patientFirstName, patientLastName)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.submitClaimToQueue
      (
        patientIndex,
        countryIndex,
        stateIndex,
        hospitalIndex,
        hospitalType,
        hospitalName,
        hospitalAddress,
        hospitalCity,
        hospitalZipCode,
        hospitalPhoneNumber,
        hospitalBillInvoiceNumber,
        note144Characters,
        claimAmount,
        ailment,
        insuranceCompanyIndex,
        insuranceCompanyName
      )
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.assignClaimToProcessor(newWallet.publicKey).rpc()

      var processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)

      await program.methods.createPatientRecord(newWallet.publicKey).rpc()
      await program.methods.createHospitalAndInsuranceCompanyRecords(newWallet.publicKey).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)

      await program.methods.approveClaim(newWallet.publicKey).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)
      
      const newHospitalBillInvoiceNumber = "abc123"
      const newClaimNote = "NEW PROCESSED CLAIM NOTE" 
      const newClaimAmount = new anchor.BN(77777) //Convert to Fixed Point
      const newAilment = "NEW AILMENT"
  
      const processor = await program.account.processorAccount.fetch(getProcessorPDA(program.provider.publicKey))

      await program.methods.editProcessedClaimAndAllRecords(
        program.provider.publicKey, 
        processor.processedClaimCount.sub(new anchor.BN(1)), 
        newHospitalBillInvoiceNumber,
        newClaimNote,
        newClaimAmount,
        newAilment)
      .rpc()
    }
  })

  it("Submits Claims To Queue, Creates All Records, Approves Claim, And Then Revokes Approval", async () => 
  {
    //Submit 100 Claims
    for(var i=1; i<=1; i++)
    {
      //Fund Wallet
      let newWallet = anchor.web3.Keypair.generate()
      let token_airdrop = await program.provider.connection.requestAirdrop(newWallet.publicKey, 
        1000 * 10002240)

      const latestBlockHash = await program.provider.connection.getLatestBlockhash()
      await program.provider.connection.confirmTransaction
      ({
        blockhash: latestBlockHash.blockhash,
        lastValidBlockHeight: latestBlockHash.lastValidBlockHeight,
        signature: token_airdrop,
      })

      //Init Submitter Account
      await program.methods.createSubmitterAccount()
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      //Init Patient Account
      const patientFirstName = "John"
      const patientLastName = "Doe"
      await program.methods.createPatientAccount(patientFirstName, patientLastName)
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.submitClaimToQueue
      (
        patientIndex,
        countryIndex,
        stateIndex,
        hospitalIndex,
        hospitalType,
        hospitalName,
        hospitalAddress,
        hospitalCity,
        hospitalZipCode,
        hospitalPhoneNumber,
        hospitalBillInvoiceNumber,
        note144Characters,
        claimAmount,
        ailment,
        insuranceCompanyIndex,
        insuranceCompanyName
      )
      .accounts({signer: newWallet.publicKey})
      .signers([newWallet])
      .rpc()

      await program.methods.assignClaimToProcessor(newWallet.publicKey).rpc()

      var processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)

      await program.methods.createPatientRecord(newWallet.publicKey).rpc()
      await program.methods.createHospitalAndInsuranceCompanyRecords(newWallet.publicKey).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)
      
      await program.methods.approveClaim(newWallet.publicKey).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)
      console.log("Undenied Claim Count: ", processorStats.undeniedClaimCount)
      console.log("Revoked Approval Count: ", processorStats.revokedApprovalCount)

      const denialReason = "Testing Approval Revoke"
      const processor = await program.account.processorAccount.fetch(getProcessorPDA(program.provider.publicKey))
      await program.methods.revokeApproval(program.provider.publicKey, processor.processedClaimCount.sub(new anchor.BN(1)), denialReason).rpc()

      processorStats = await program.account.processorStats.fetch(getprocessorStatsPDA())
      console.log("Processed Claim Count: ", processorStats.processedClaimCount)
      console.log("Approved Claim Count: ", processorStats.approvedClaimCount)
      console.log("Undenied Claim Count: ", processorStats.undeniedClaimCount)
      console.log("Revoked Approval Count: ", processorStats.revokedApprovalCount)
    }

    /*while(true)
    {
      await sleepFunction()
    }*/
  })

  const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms))
  var counter = 0
  async function sleepFunction() {
    console.log('Start sleep: ', counter)
    await sleep(5000) // Sleep for 5 seconds
    console.log('End sleep: ', counter)
    counter += 1
  }

  function getM4AProtocolCEOAccountPDA()
  {
    const [m4aProtocolCEOPDA] = anchor.web3.PublicKey.findProgramAddressSync
    (
      [
        new TextEncoder().encode("m4aProtocolCEO")
      ],
      program.programId
    )
    return m4aProtocolCEOPDA
  }

  function getM4AProtocolPDA()
  {
    const [m4aProtocolPDA] = anchor.web3.PublicKey.findProgramAddressSync
    (
      [
        new TextEncoder().encode("m4aProtocol")
      ],
      program.programId
    )
    return m4aProtocolPDA
  }

  function getprocessorStatsPDA()
  {
    const [processorStatsPDA] = anchor.web3.PublicKey.findProgramAddressSync
    (
      [
        new TextEncoder().encode("processorStats")
      ],
      program.programId
    )
    return processorStatsPDA
  }

  function getPatientPDA(submitterAddress: anchor.web3.PublicKey, patientIndex: number)
  {
    const [patientPDA] = anchor.web3.PublicKey.findProgramAddressSync
    (
      [
        new TextEncoder().encode("patient"),
        submitterAddress.toBuffer(),
        new anchor.BN(patientIndex).toBuffer('le', 1)
      ],
      program.programId
    )
    return patientPDA
  }

  function getProcessorPDA(processorAddress: anchor.web3.PublicKey)
  {
    const [processorPDA] = anchor.web3.PublicKey.findProgramAddressSync
    (
      [
        new TextEncoder().encode("processor"),
        processorAddress.toBuffer()
      ],
      program.programId
    )
    return processorPDA
  }

  function getClaimPDA(submitterAddress: anchor.web3.PublicKey )
  {
    const [claimPDA] = anchor.web3.PublicKey.findProgramAddressSync
    (
      [
        new TextEncoder().encode("claim"),
        submitterAddress.toBuffer()
      ],
      program.programId
    )
    return claimPDA
  }

  function getClaimQueuePDA()
  {
    const [claimQueuePDA] = anchor.web3.PublicKey.findProgramAddressSync
    (
      [
        utf8.encode("claimQueue"),
      ],
      program.programId
    )
    return claimQueuePDA
  }

  function getClaimHistoryPDA()
  {
    const [claimHistoryPDA] = anchor.web3.PublicKey.findProgramAddressSync
    (
      [
        utf8.encode("claimHistory"),
      ],
      program.programId
    )
    return claimHistoryPDA
  }

  function getClaimHistoryChunkPDA(chunkId: number)
  {
    const [claimHistoryChunkPDA] = anchor.web3.PublicKey.findProgramAddressSync
    (
      [
        utf8.encode("claimHistoryChunk"),
        new anchor.BN(chunkId).toBuffer('le', 4)
      ],
      program.programId
    )
    return claimHistoryChunkPDA
  }

  function getProcessedClaimPDA(processedClaimIndex: number)
  {
    const [claimHistoryChunkPDA] = anchor.web3.PublicKey.findProgramAddressSync
    (
      [
        utf8.encode("processedClaim"),
        new anchor.web3.PublicKey("93AWVuZKVhNQ4Uky3oFFwNewTgMs6T4aenZ4efom4gUw").toBuffer(),
        new anchor.BN(processedClaimIndex).toBuffer('le', 4)
      ],
      program.programId
    )
    return claimHistoryChunkPDA
  }

  function getInsuranceCompanyPDA(index: number)
  {
    const [insuranceCompanyPDA] = anchor.web3.PublicKey.findProgramAddressSync
    (
      [
        utf8.encode("insuranceCompany"),
        new anchor.BN(index).toBuffer('le', 4)
      ],
      program.programId
    )
    return insuranceCompanyPDA
  }

  const chunk = (arr: any[], size: number) => Array.from
  (
    { length: Math.ceil(arr.length / size) }, (_, i) => 
    arr.slice(i * size, i * size + size)
  )

  function getNewTime()
  {
    var newDate = new Date()
  
    return newDate.toLocaleTimeString('en-US', 
    { timeZone: 'America/New_York', 
      timeZoneName: "short"
    })
  }

  function getNewDate()
  {
    var newDate = new Date()
  
    return newDate.toLocaleDateString('en-US', 
    { 
      weekday: 'short',
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    })
  }
})