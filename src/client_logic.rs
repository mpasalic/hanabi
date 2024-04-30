use crate::model::{CardFace, CardSuit};

#[derive(Clone)]
pub enum HintState {
    ChoosingPlayer,
    ChoosingHintType { player_index: u8 },
    // ChoosingCard {
    //     player_index: u8,
    //     hint_type: HintBuilderType,
    // },
    ChoosingSuit { player_index: u8 },
    ChoosingFace { player_index: u8 },
}

#[derive(Clone, Copy)]
pub enum HintBuilderType {
    Suite,
    Face,
}

pub enum AppAction {
    StartHint,
    Undo,
    SelectPlayer { player_index: u8 },
    SelectHintType { hint_type: HintBuilderType },
    SelectSuit(CardSuit),
    SelectFace(CardFace),
}

#[derive(Clone)]
pub enum CommandBuilder {
    Empty,
    Hint(HintState),
}

struct App {
    current_command: CommandBuilder,
}

fn process_app_action(state: App, action: AppAction) -> App {
    use AppAction as A;
    use CommandBuilder as C;
    let builder = match (state.current_command, action) {
        (C::Empty, A::StartHint) => C::Hint(HintState::ChoosingPlayer),

        (C::Hint(HintState::ChoosingPlayer), A::SelectPlayer { player_index }) => {
            C::Hint(HintState::ChoosingHintType { player_index })
        }

        (
            C::Hint(HintState::ChoosingHintType { player_index }),
            A::SelectHintType { hint_type },
        ) => C::Hint(match hint_type {
            HintBuilderType::Suite => HintState::ChoosingSuit { player_index },
            HintBuilderType::Face => HintState::ChoosingFace { player_index },
        }),

        // TODO produce a command
        (C::Hint(HintState::ChoosingSuit { player_index }), A::SelectSuit(suit)) => C::Empty,

        // TODO produce a command
        (C::Hint(HintState::ChoosingFace { player_index }), A::SelectFace(face)) => C::Empty,

        // ----- Undo -----
        (C::Hint(HintState::ChoosingPlayer), A::Undo) => C::Empty,

        (C::Hint(HintState::ChoosingHintType { player_index }), A::Undo) => {
            C::Hint(HintState::ChoosingPlayer)
        }

        (
            C::Hint(HintState::ChoosingSuit { player_index })
            | C::Hint(HintState::ChoosingFace { player_index }),
            A::Undo,
        ) => C::Hint(HintState::ChoosingHintType { player_index }),

        // ------ other wise do nothing -------
        (builder, _) => builder,
    };

    App {
        current_command: builder,
    }
}
