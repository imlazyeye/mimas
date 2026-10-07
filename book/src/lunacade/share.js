// Sharing a cart. The cart is written as json, compressed with the browser's own deflate and
// turned into base64url, which goes after `#cart=` in a link. The part after a `#` never reaches a
// server.

// the most a cart or a link can unpack to (a small link can't fill the tab)
const LIMIT = 4 * 1024 * 1024;

export function validFiles(files) {
    return (
        files !== null &&
        typeof files === 'object' &&
        !Array.isArray(files) &&
        Object.values(files).every((text) => typeof text === 'string')
    );
}

export async function encodeCart(cart) {
    const bytes = new TextEncoder().encode(JSON.stringify(cart));
    if (bytes.length > LIMIT) throw new Error('the cart is too big to share');
    return toBase64(await pipe(bytes, new CompressionStream('deflate-raw')));
}

// takes the text with or without the `#cart=`, and throws if it isn't a cart
export async function decodeCart(fragment) {
    const compressed = fromBase64(fragment.replace(/^#?(?:cart=)?/, ''));
    const bytes = await pipe(compressed, new DecompressionStream('deflate-raw'));
    const cart = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
    if (!validFiles(cart?.files)) throw new Error('the link has no cart in it');
    return { id: typeof cart.id === 'string' ? cart.id : 'shared', files: cart.files };
}

async function pipe(bytes, transform) {
    const reader = new Blob([bytes]).stream().pipeThrough(transform).getReader();
    const chunks = [];
    let size = 0;
    for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        size += value.length;
        if (size > LIMIT) {
            await reader.cancel();
            throw new Error('the link is too big');
        }
        chunks.push(value);
    }
    const out = new Uint8Array(size);
    let at = 0;
    for (const chunk of chunks) {
        out.set(chunk, at);
        at += chunk.length;
    }
    return out;
}

// btoa wants a string with a character for each byte, and spreading a big array into a call
// overflows the stack (hence the slices)
function toBase64(bytes) {
    let binary = '';
    for (let i = 0; i < bytes.length; i += 0x8000) {
        binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
    }
    return btoa(binary).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/, '');
}

function fromBase64(text) {
    if (!/^[\w-]*={0,2}$/.test(text)) throw new Error('the link is not base64url');
    const binary = atob(text.replaceAll('-', '+').replaceAll('_', '/'));
    return Uint8Array.from(binary, (char) => char.charCodeAt(0));
}
