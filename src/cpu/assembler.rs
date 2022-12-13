/*
    IDEA: Translate a string containing assembly code into a list of bytes, which can be executed by the CPU
    Things that need to be done:

    - Parse the string containing the instructions into an abstract syntax tree
        - For example, "LDA #00" should be parsed into the following:
            * "LDA": a "instruction" node
            * "#": a "(immediate) addressing mode" node
            * "00": a "address" node

    - Translate the abstract syntax tree into a list of bytes
        - Look up the "instruction" nodes text in the instruction set.
            - TODO: Explain function with which this is done
            - If no instruction is found with that name, throw an error

        - Fetch the opcode of the instruction
            - Check if the instruction has any arguments
                - If not, just add the only opcode to the list of bytes and skip the rest of the steps

            - Otherwise, If the instruction does not support any arguments, throw an error
                - Otherwise, check if the next node is an "addressing mode" which the instruction supports
                    - If not, throw an error

            - Look up the opcode (number identifying the instruction) for the instruction, based on the "addressing mode"
                - For example, the opcode for "LDA #" would be opcode "0xa9" (LDA immediate).
                    - Note that this would throw an error, as an "address" node needs to go after the "addressing mode" node

            - Append the opcode and its arguments to the list of bytes
                - For example, "LDA #00" would be translated into the following bytes:
                    * 0xa9
                    * 0x00

        - Begin all over again until the end of the string is reached
*/
