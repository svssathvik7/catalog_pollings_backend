use mongodb::Collection;

use super::reg_state::RegState;

pub struct DB{
    pub reg_states: Collection<RegState>
}