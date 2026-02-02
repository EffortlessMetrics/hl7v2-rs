import sys
import os

def replace_in_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    # Fix map_clone
    new_content = content.replace('.iter().map(|s| *s)', '.iter().copied()')

    # Fix get_delimiters_from_file_batch collapsible if
    old_block = """fn get_delimiters_from_file_batch(file_batch: &FileBatch) -> Delims {
    // Try to get delimiters from the first message in the first batch
    if let Some(first_batch) = file_batch.batches.first() {
        if let Some(first_message) = first_batch.messages.first() {
            return first_message.delims.clone();
        }
    }
    // Fallback to default delimiters
    Delims::new_default()
}"""
    new_block = """fn get_delimiters_from_file_batch(file_batch: &FileBatch) -> Delims {
    // Try to get delimiters from the first message in the first batch
    if let Some(first_message) = file_batch.batches.first().and_then(|b| b.messages.first()) {
        return first_message.delims.clone();
    }
    // Fallback to default delimiters
    Delims::new_default()
}"""

    if old_block in new_content:
        new_content = new_content.replace(old_block, new_block)
    else:
        print("Could not find get_delimiters block")
        # Try to find it loosely or print what is there
        start = new_content.find("fn get_delimiters_from_file_batch")
        if start != -1:
            print("Found function start, content around it:")
            print(new_content[start:start+400])

    with open(filepath, 'w') as f:
        f.write(new_content)

if __name__ == "__main__":
    replace_in_file("crates/hl7v2-core/src/lib.rs")
