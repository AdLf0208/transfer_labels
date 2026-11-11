// Runs `/transfer_labels …` for the acting player (API 8).
fn transfer_labels_perform_command(
    game: &mut crate::game::Game<'_>,
    player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    crate::game::transfer_labels_state::on_perform_command(game, player, command, ran);
}
