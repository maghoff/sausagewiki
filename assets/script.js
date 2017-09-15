function autosizeTextarea(textarea, shadow) {
    shadow.style.width = textarea.clientWidth + "px";
    shadow.value = textarea.value;
    textarea.style.height = shadow.scrollHeight + "px";
}

function queryArgsFromForm(form) {
    const items = [];
    for (const {name, value} of form.elements) {
        if (!name) continue;
        items.push(encodeURIComponent(name) + '=' + encodeURIComponent(value));
    }
    return items.join('&');
}

let hasBeenOpen = false;
function openEditor() {
    const article = document.querySelector("article");
    const rendered = article.querySelector(".rendered");
    const editor = article.querySelector(".editor");
    const textarea = editor.querySelector('textarea[name="body"]');
    const shadow = editor.querySelector('textarea.shadow-control');
    const form = editor.querySelector("form");
    const cancel = editor.querySelector('.cancel');

    const footer = document.querySelector("footer");
    const revision = footer.querySelector(".revision");
    const lastUpdated = footer.querySelector(".last-updated");

    textarea.style.height = rendered.clientHeight + "px";

    article.classList.add('edit');

    autosizeTextarea(textarea, shadow);

    textarea.focus();

    if (hasBeenOpen) return;
    hasBeenOpen = true;

    textarea.addEventListener('input', () => autosizeTextarea(textarea, shadow));
    window.addEventListener('resize', () => autosizeTextarea(textarea, shadow));

    form.addEventListener("submit", function (ev) {
        ev.preventDefault();
        ev.stopPropagation();

        (async function () {
            const body = queryArgsFromForm(form);
            textarea.disabled = true;

            const response = await fetch(
                form.getAttribute("action"),
                {
                    method: 'PUT',
                    headers: {
                        "Content-Type": "application/x-www-form-urlencoded"
                    },
                    body: body,
                }
            );

            if (!response.ok) throw new Error("Unexpected status code (" + response.status + ")");

            const result = await response.json();
            form.elements.base_revision.value = result.revision;
            revision.textContent = result.revision;
            lastUpdated.textContent = result.created;
            rendered.innerHTML = result.rendered;
            article.classList.remove('edit');

            textarea.disabled = false;
        }()
        .catch(err => {
            textarea.disabled = false;
            console.error(err);
            alert(err);
        }));
    });

    cancel.addEventListener('click', function (ev) {
        ev.preventDefault();
        ev.stopPropagation();

        article.classList.remove('edit');
        form.reset();
    });
}

document
    .getElementById("openEditor")
    .addEventListener("click", function (ev) {
        ev.preventDefault();
        ev.stopPropagation();

        openEditor();
    })
