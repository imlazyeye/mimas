// `h('div', { class: 'row', title: 'x' }, child, 'text')` makes an element. A child that's false,
// null or undefined is left out, so a part can be conditional.
export function h(tag, props = {}, ...children) {
    const node = document.createElement(tag);
    for (const [name, value] of Object.entries(props)) {
        if (value !== false && value !== null && value !== undefined) node.setAttribute(name, value);
    }
    node.append(...children.filter((child) => child !== false && child != null));
    return node;
}

export function button(label, onClick, props = {}) {
    const node = h('button', { type: 'button', ...props }, label);
    node.addEventListener('click', onClick);
    return node;
}
