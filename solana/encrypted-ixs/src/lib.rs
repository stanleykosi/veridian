use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    // This is a placeholder encrypted instruction
    // It will be replaced with actual game logic in later steps
    pub struct PlaceholderInput {
        value: u8,
    }

    #[instruction]
    pub fn placeholder(input_ctxt: Enc<Shared, PlaceholderInput>) -> Enc<Shared, u8> {
        let input = input_ctxt.to_arcis();
        let result = input.value;
        input_ctxt.owner.from_arcis(result)
    }
}