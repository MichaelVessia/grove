use super::*;

impl UiCommand {
    pub(super) fn help_hints_for(context: HelpHintContext) -> Vec<UiCommand> {
        Self::all()
            .iter()
            .filter_map(|command| command.help_hint(context).map(|_| *command))
            .collect()
    }

    pub(super) fn help_hint(self, context: HelpHintContext) -> Option<&'static HelpHintSpec> {
        self.meta()
            .help_hints
            .iter()
            .find(|hint| hint.context == context)
    }
}
