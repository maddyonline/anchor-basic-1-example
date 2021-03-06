const assert = require("assert");
const anchor = require("@project-serum/anchor");
const serumCmn = require("@project-serum/common");
const { TOKEN_PROGRAM_ID } = require("@solana/spl-token");
const { SystemProgram } = anchor.web3;

describe("basic-1", () => {
  // Use a local provider.
  const provider = anchor.Provider.local();

  // Configure the client to use the local cluster.
  anchor.setProvider(provider);

  let mint = null;
  let god = null;
  let receiver = null;

  it("Sets up initial test state", async () => {
    const program = anchor.workspace.Xbasic1;
    const [_mint, _god] = await serumCmn.createMintAndVault(
      program.provider,
      new anchor.BN(1_000_000)
    );
    mint = _mint;
    god = _god;

    receiver = await serumCmn.createTokenAccount(
      program.provider,
      mint,
      program.provider.wallet.publicKey
    );
  });
  const check = anchor.web3.Keypair.generate();
  const vault = anchor.web3.Keypair.generate();

  let checkSigner = null;

  it("Creates a check!", async () => {
    const program = anchor.workspace.Xbasic1;
    let [_checkSigner, nonce] = await anchor.web3.PublicKey.findProgramAddress(
      [check.publicKey.toBuffer()],
      program.programId
    );
    checkSigner = _checkSigner;

    await program.rpc.createCheck(new anchor.BN(100), "gm", nonce, {
      accounts: {
        check: check.publicKey,
        vault: vault.publicKey,
        checkSigner,
        from: god,
        to: receiver,
        owner: program.provider.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [check, vault],
      instructions: [
        await program.account.check.createInstruction(check, 300),
        ...(await serumCmn.createTokenAccountInstrs(
          program.provider,
          vault.publicKey,
          mint,
          checkSigner
        )),
      ],
    });
  });


  let visitorState, visitorBump;
  const visitor = anchor.web3.Keypair.generate();
  before(async () => {
    const program = anchor.workspace.Xbasic1;
    [visitorState, visitorBump] = await anchor.web3.PublicKey.findProgramAddress(
      [visitor.publicKey.toBuffer(), "1"],
      program.programId
    );
  });

  it("It works!", async () => {
    const program = anchor.workspace.Xbasic1;
    await program.rpc.introduceYourself(new anchor.BN(visitorBump),
      {
        accounts: {
          payer: provider.wallet.payer.publicKey,
          visitor: visitor.publicKey,
          visitorState: visitorState,
          systemProgram: anchor.web3.SystemProgram.programId
        },
        signers: [visitor]
      }
    );

    let visitorStateAccount = await program.account.visitorState.fetch(visitorState);
    assert.equal(1, visitorStateAccount.visitCount.toNumber())

    async function visit() {
      await provider.connection.confirmTransaction(
        await program.rpc.visit(
          {
            accounts: {
              visitor: visitor.publicKey,
              visitorState: visitorState
            },
            signers: [visitor]
          }
        ),
        "finalized"
      );
    }

    console.log("About to visit again, this takes a while for solana to finalize...");
    await visit();
    visitorStateAccount = await program.account.visitorState.fetch(visitorState);
    assert.equal(2, visitorStateAccount.visitCount.toNumber());

  });


  it("Cashes a check", async () => {
    const program = anchor.workspace.Xbasic1;
    await program.rpc.cashCheck({
      accounts: {
        check: check.publicKey,
        vault: vault.publicKey,
        checkSigner: checkSigner,
        to: receiver,
        owner: program.provider.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });

    const checkAccount = await program.account.check.fetch(check.publicKey);
    assert.ok(checkAccount.burned === true);

    let vaultAccount = await serumCmn.getTokenAccount(
      program.provider,
      checkAccount.vault
    );
    assert.ok(vaultAccount.amount.eq(new anchor.BN(0)));

    let receiverAccount = await serumCmn.getTokenAccount(
      program.provider,
      receiver
    );
    assert.ok(receiverAccount.amount.eq(new anchor.BN(100)));
  });

  it("Creates and initializes an account in a single atomic transaction (simplified)", async () => {
    // #region code-simplified
    // The program to execute.
    const program = anchor.workspace.Xbasic1;

    // The Account to create.
    const myAccount = anchor.web3.Keypair.generate();

    // Create the new account and initialize it with the program.
    // #region code-simplified
    await program.rpc.initialize(new anchor.BN(1234), {
      accounts: {
        myAccount: myAccount.publicKey,
        user: provider.wallet.publicKey,
        systemProgram: SystemProgram.programId,
      },
      signers: [myAccount],
    });
    // #endregion code-simplified

    // Fetch the newly created account from the cluster.
    const account = await program.account.myAccount.fetch(myAccount.publicKey);

    // Check it's state was initialized.
    assert.ok(account.data.eq(new anchor.BN(1234)));

    // Store the account for the next test.
    _myAccount = myAccount;
  });

  it("Updates a previously created account", async () => {
    const myAccount = _myAccount;

    // #region update-test

    // The program to execute.
    const program = anchor.workspace.Xbasic1;

    // Invoke the update rpc.
    await program.rpc.update(new anchor.BN(4321), {
      accounts: {
        myAccount: myAccount.publicKey,
      },
    });

    // Fetch the newly updated account.
    const account = await program.account.myAccount.fetch(myAccount.publicKey);

    // Check it's state was mutated.
    assert.ok(account.data.eq(new anchor.BN(4321)));

    // #endregion update-test
  });
});