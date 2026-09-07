// renaming the themes because i'm extra
window.addEventListener('DOMContentLoaded', () => {
  const labels = { light: 'Light', navy: 'Dark' };
  for (const [id, label] of Object.entries(labels)) {
    const btn = document.getElementById(id);
    if (btn) btn.textContent = label;
  }

  // mdBook renders the menu title as plain text; make it a home link
  const title = document.querySelector('.menu-title');
  if (title) {
    const a = document.createElement('a');
    a.href = path_to_root + 'index.html';
    a.append(...title.childNodes);
    title.append(a);
  }
});
