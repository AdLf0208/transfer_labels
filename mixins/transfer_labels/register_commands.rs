// Declares `/transfer_labels …`'s usage lines (API 8).
fn transfer_labels_register_commands(commands: &mut Vec<NativeCommand>) {
    crate::game::transfer_labels_state::on_register_commands(commands);
}
