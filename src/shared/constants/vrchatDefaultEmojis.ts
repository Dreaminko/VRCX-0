const defaultEmojiCatalog = Object.freeze([
    { name: 'Angry', previewFile: 'Angry.webp' },
    { name: 'Blushing', previewFile: 'Blush.webp' },
    { name: 'Crying', previewFile: 'Crying.webp' },
    { name: 'Frown', previewFile: 'Frown.webp' },
    { name: 'Hand Wave', previewFile: 'Handwave.webp' },
    { name: 'Hang Ten', previewFile: 'Summer_Hangten.webp' },
    { name: 'In Love', previewFile: 'Inlove.webp' },
    { name: 'Jack O Lantern', previewFile: 'Fall_Jackolantern.webp' },
    { name: 'Kiss', previewFile: 'Kiss.webp' },
    { name: 'Laugh', previewFile: 'Laugh.webp' },
    { name: 'Skull', previewFile: 'Fall_Skull.webp' },
    { name: 'Smile', previewFile: 'Smile.webp' },
    { name: 'Spooky Ghost', previewFile: 'Fall_Ghost.webp' },
    { name: 'Stoic', previewFile: 'Stoic.webp' },
    { name: 'Sunglasses', previewFile: 'Sunglasses.webp' },
    { name: 'Thinking', previewFile: 'Thinking.webp' },
    { name: 'Thumbs Down', previewFile: 'Dislike.webp' },
    { name: 'Thumbs Up', previewFile: 'Like.webp' },
    { name: 'Tongue Out', previewFile: 'Tongue.webp' },
    { name: 'Wow', previewFile: 'Wow.webp' },
    { name: 'Arrow Point', previewFile: 'Accessibility_Arrow.webp' },
    { name: "Can't see", previewFile: 'Accessibility_Blind.webp' },
    { name: 'Hourglass', previewFile: 'Accessibility_Hourglass.webp' },
    { name: 'Keyboard', previewFile: 'Accessibility_Keyboard.webp' },
    {
        name: 'No Headphones',
        previewFile: 'Accessibility_Deafened.webp'
    },
    { name: 'No Mic', previewFile: 'Accessibility_Muted.webp' },
    { name: 'Portal', previewFile: 'Accessibility_Portal.webp' },
    { name: 'Shush', previewFile: 'Accessibility_Shush.webp' },
    { name: 'Bats', previewFile: 'Fall_Bat.webp' },
    { name: 'Cloud', previewFile: 'Cloud.webp' },
    { name: 'Fire', previewFile: 'Fire.webp' },
    { name: 'Snow Fall', previewFile: 'Winter_Snowflake.webp' },
    {
        name: 'Snowball',
        previewFile: 'Snowball_-_emoji_animation_type.gif'
    },
    { name: 'Splash', previewFile: 'Summer_Splash.webp' },
    { name: 'Web', previewFile: 'Fall_Web.webp' },
    { name: 'Beer', previewFile: 'Beer.webp' },
    { name: 'Candy', previewFile: 'Fall_Candy.webp' },
    { name: 'Candy Cane', previewFile: 'Winter_Candycane.webp' },
    { name: 'Candy Corn', previewFile: 'Fall_CandyCorn.webp' },
    { name: 'Champagne', previewFile: 'Winter_Champagneclink.webp' },
    { name: 'Drink', previewFile: 'Summer_Coconut_Drink.webp' },
    { name: 'Gingerbread', previewFile: 'Winter_Gingerbreadman.webp' },
    { name: 'Ice Cream', previewFile: 'Summer_Icecream.webp' },
    { name: 'Pineapple', previewFile: 'Summer_Pineapple.webp' },
    { name: 'Pizza', previewFile: 'Pizza.webp' },
    { name: 'Tomato', previewFile: 'Tomato.webp' },
    { name: 'Beachball', previewFile: 'Summer_Beachball.webp' },
    { name: 'Coal', previewFile: 'Winter_Coal.webp' },
    { name: 'Confetti', previewFile: 'Winter_ConfettiPopper.webp' },
    { name: 'Gift', previewFile: 'Gift.webp' },
    { name: 'Gifts', previewFile: 'Winter_Gifts.webp' },
    { name: 'Life Ring', previewFile: 'Lifering.webp' },
    { name: 'Mistletoe', previewFile: 'Winter_Mistletoe.webp' },
    { name: 'Money', previewFile: 'Money.webp' },
    { name: 'Neon Shades', previewFile: 'Summer_Neonshades.webp' },
    { name: 'Sun Lotion', previewFile: 'Summer_Sunlotion.webp' },
    { name: 'Boo', previewFile: 'Fall_BOO.webp' },
    { name: 'Broken Heart', previewFile: 'Brokenheart.webp' },
    { name: 'Exclamation', previewFile: 'Exclaim.webp' },
    { name: 'Go', previewFile: 'Go.webp' },
    { name: 'Heart', previewFile: 'Love.webp' },
    { name: 'Music Note', previewFile: 'Music.webp' },
    { name: 'Question', previewFile: 'Question.webp' },
    { name: 'Stop', previewFile: 'Stop.webp' },
    { name: 'Zzz', previewFile: 'ZZZZ.webp' }
]);

function defaultEmojiId(name: string): string {
    return `default_${name.replace(/ /g, '_').toLowerCase()}`;
}

export const vrchatDefaultEmojis = Object.freeze(
    defaultEmojiCatalog.map(({ name, previewFile }) => ({
        id: defaultEmojiId(name),
        name,
        previewUrl: `https://wiki-files.vrchat.com/${previewFile}`
    }))
);

const defaultEmojiById = new Map(
    vrchatDefaultEmojis.map((emoji) => [emoji.id, emoji])
);

export function defaultEmojiName(emojiId: string): string {
    return (
        defaultEmojiById.get(emojiId)?.name ??
        emojiId.replace(/^default_/, '').replace(/_/g, ' ')
    );
}

export function defaultEmojiPreviewUrl(emojiId: string): string {
    return defaultEmojiById.get(emojiId)?.previewUrl ?? '';
}
