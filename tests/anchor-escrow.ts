import * as anchor from "@project-serum/anchor";
import { IdlAccounts, Program } from "@project-serum/anchor";
import { AnchorEscrow } from "../target/types/anchor_escrow";
import { Token, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import {
  Keypair,
  LAMPORTS_PER_SOL, PublicKey, SystemProgram
} from '@solana/web3.js';
import { assert } from "chai";

type EscrowAccount = IdlAccounts<AnchorEscrow>["escrowAccount"];

describe("anchor-escrow", () => {
  // Configure the client to use the local cluster.
  const provier = anchor.AnchorProvider.env();
  anchor.setProvider(provier);

  const program = anchor.workspace.AnchorEscrow as Program<AnchorEscrow>;

  let mintA: Token = null;
  let mintB: Token = null;
  let initializerTokenAccountA: PublicKey = null;
  let initializerTokenAccountB: PublicKey = null;
  let takerTokenAccountA: PublicKey = null;
  let takerTokenAccountB: PublicKey = null;

  let pda: PublicKey = null;

  const takerAmount = 1000;
  const initializerAmount = 500;

  const escrowAccount = anchor.web3.Keypair.generate();
  const payer = anchor.web3.Keypair.generate();
  const mintAuthority = anchor.web3.Keypair.generate();

  it("Initialize Escrow State", async () => {
    // airdroping tokens to a payer
    await provier.connection.confirmTransaction(
      await provier.connection.requestAirdrop(
        payer.publicKey, LAMPORTS_PER_SOL * 10
      ), "confirmed"
    );

    mintA = await Token.createMint(
      provier.connection,
      payer,
      mintAuthority.publicKey,
      null,
      0,
      TOKEN_PROGRAM_ID
    );

    mintB = await Token.createMint(
      provier.connection,
      payer,
      mintAuthority.publicKey,
      null,
      0,
      TOKEN_PROGRAM_ID
    );

    initializerTokenAccountA = await mintA.createAccount(provier.wallet.publicKey);
    takerTokenAccountA = await mintA.createAccount(provier.wallet.publicKey);

    initializerTokenAccountB = await mintB.createAccount(provier.wallet.publicKey);
    takerTokenAccountB = await mintB.createAccount(provier.wallet.publicKey);

    await mintA.mintTo(
      initializerTokenAccountA,
      mintAuthority.publicKey,
      [mintAuthority],
      initializerAmount
    );

    await mintB.mintTo(
      takerTokenAccountB,
      mintAuthority.publicKey,
      [mintAuthority],
      takerAmount
    );

    let _initializerTokenAccountA = await mintA.getAccountInfo(initializerTokenAccountA);
    let _takerTokenAccountB = await mintB.getAccountInfo(takerTokenAccountB);

    assert.strictEqual(
      _initializerTokenAccountA.amount.toNumber(),
      initializerAmount
    );

    assert.strictEqual(
      _takerTokenAccountB.amount.toNumber(),
      takerAmount
    );
  });

  it("Initialize Escrow Account",async () => {
    await program.rpc.initialize(
      new anchor.BN(initializerAmount), 
      new anchor.BN(takerAmount), {
      accounts: {
        initializer: provier.wallet.publicKey,
        initializerDepositTokenAccount: initializerTokenAccountA,
        initializerReceiveTokenAccount: initializerTokenAccountB,
        escrowAccount: escrowAccount.publicKey,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
      signers: [escrowAccount]
    });

    const [_pda, _bump_seed] = await PublicKey.findProgramAddress(
      [Buffer.from(anchor.utils.bytes.utf8.encode("escrow"))],
      program.programId
    );

    pda = _pda;

    let _initializerTokenAccountA = await mintA.getAccountInfo(initializerTokenAccountA);
    let _escrowAccount: EscrowAccount = await program.account.escrowAccount.fetch(escrowAccount.publicKey);

    assert.isTrue(_initializerTokenAccountA.owner.equals(pda));
    assert.isTrue(
      _escrowAccount.initializerKey.equals(provier.wallet.publicKey)
    );
    assert.strictEqual(
      _escrowAccount.initializeAmount.toNumber(),
      initializerAmount
    );
    assert.strictEqual(_escrowAccount.takerAmount.toNumber(), takerAmount);
    assert.isTrue(_escrowAccount.initializerDepositTokenAccount.equals(initializerTokenAccountA));
    assert.isTrue(_escrowAccount.initializerReceiveTokenAccount.equals(initializerTokenAccountB));
  });

  it("Exchange Escrow", async () => {
    await program.rpc.exchange({
      accounts: {
        taker: provier.wallet.publicKey,
        takerDepositTokenAccount: takerTokenAccountB,
        takerReceiveTokenAccount: takerTokenAccountA,
        pdaDepositTokenAccount: initializerTokenAccountA,
        initializerReceiveTokenAccount: initializerTokenAccountB,
        initializerMainAccount: provier.wallet.publicKey,
        escrowAccount: escrowAccount.publicKey,
        pdaAccount: pda,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });

    let _takerTokenAccountA = await mintA.getAccountInfo(takerTokenAccountA);
    let _takerTokenAccountB = await mintB.getAccountInfo(takerTokenAccountB);
    let _initializerTokenAccountA = await mintA.getAccountInfo(initializerTokenAccountA);
    let _initializerTokenAccountB = await mintB.getAccountInfo(initializerTokenAccountB);

    assert.isTrue(_takerTokenAccountA.owner.equals(provier.wallet.publicKey));
    assert.strictEqual(_takerTokenAccountA.amount.toNumber(), initializerAmount);
    assert.strictEqual(_takerTokenAccountB.amount.toNumber(), 0);
    assert.strictEqual(_initializerTokenAccountB.amount.toNumber(), takerAmount);
    assert.strictEqual(_initializerTokenAccountA.amount.toNumber(), 0);
  });

  let newEscrow = Keypair.generate();
  it("Cancel Escrow",async () => {
    await mintA.mintTo(
      initializerTokenAccountA,
      mintAuthority.publicKey,
      [mintAuthority],
      initializerAmount
    );

    await program.rpc.initialize(
      new anchor.BN(initializerAmount),
      new anchor.BN(takerAmount),
      {
        accounts: {
          initializer: provier.wallet.publicKey,
          initializerDepositTokenAccount: initializerTokenAccountA,
          initializerReceiveTokenAccount: initializerTokenAccountB,
          escrowAccount: newEscrow.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [newEscrow],
      }
    )

    let _initializerTokenAccountA = await mintA.getAccountInfo(initializerTokenAccountA);
    
    assert.isTrue(_initializerTokenAccountA.owner.equals(pda));

    await program.rpc.cancelEscrow({
      accounts: {
        initializer: provier.wallet.publicKey,
        pdaDepositTokenAccount: initializerTokenAccountA,
        pdaAccount: pda,
        escrowAccount: newEscrow.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      }
    });
    
    _initializerTokenAccountA = await mintA.getAccountInfo(initializerTokenAccountA);
    assert.isTrue(_initializerTokenAccountA.owner.equals(provier.wallet.publicKey));
    assert.strictEqual(_initializerTokenAccountA.amount.toNumber(), initializerAmount);
  });
});
