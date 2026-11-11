// Declares the command's name to the chat tree's completion (API 1).
fn transfer_labels_commands(commands: &mut Vec<&'static str>) {
    crate::game::transfer_labels_state::on_commands(commands);
}
