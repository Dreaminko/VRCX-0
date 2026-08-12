mod my_avatars;
mod remote_mutations;

pub use my_avatars::{
    get_my_avatar_by_id, get_my_avatars, MyAvatarByIdInput, MyAvatarsDeps, MyAvatarsInput,
};
pub use remote_mutations::{
    delete_avatar, execute_avatar_remote_mutation, save_avatar, select_avatar,
    AvatarRemoteMutationDeps, AvatarSelectionMutationOutcome,
};
