import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PositionManagement } from "../target/types/position_management";

describe("position-management", () => {
  // Use local validator
  const provider = anchor.AnchorProvider.local();
  anchor.setProvider(provider);

  const program = anchor.workspace.PositionManagement as Program<PositionManagement>;

  it("Program loads successfully", async () => {
    console.log("Program ID:", program.programId.toBase58());
  });
});
