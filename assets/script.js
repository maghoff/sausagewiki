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
    const container = document.querySelector(".container");
    const rendered = container.querySelector(".rendered");
    const editor = container.querySelector(".editor");
    const textarea = editor.querySelector('textarea[name="body"]');
    const shadow = editor.querySelector('textarea.shadow-control');
    const form = editor.querySelector("form");
    const cancel = editor.querySelector('.cancel');

    const footer = document.querySelector("footer");
    const articleId = footer.querySelector(".article-id");
    const revision = footer.querySelector(".revision");
    const lastUpdated = footer.querySelector(".last-updated");

    textarea.style.height = rendered.clientHeight + "px";

    container.classList.add('edit');

    autosizeTextarea(textarea, shadow);

    textarea.focus();

    if (hasBeenOpen) return;
    hasBeenOpen = true;

    textarea.addEventListener('input', () => autosizeTextarea(textarea, shadow));
    window.addEventListener('resize', () => autosizeTextarea(textarea, shadow));

    form.addEventListener("submit", function (ev) {
        ev.preventDefault();
        ev.stopPropagation();

        const body = queryArgsFromForm(form);
        textarea.disabled = true;

        fetch(
            form.getAttribute("action"),
            {
                method: 'PUT',
                headers: {
                    "Content-Type": "application/x-www-form-urlencoded"
                },
                body: body,
                credentials: "same-origin",
            }
        ).then(response => {
            if (!response.ok) throw new Error("Unexpected status code (" + response.status + ")");

            return response.json();
        }).then(result => {
            // Update url-bar, page title and footer
            window.history.replaceState(null, result.title, result.slug == "" ? "." : result.slug);
            document.querySelector("title").textContent = result.title;
            if (result.article_id != null) articleId.textContent = result.article_id;
            revision.textContent = result.revision;
            lastUpdated.textContent = result.created;

            // Update body:
            rendered.innerHTML = result.rendered;

            // Update form:
            form.elements.base_revision.value = result.revision;
            for (const element of form.elements) {
                element.defaultValue = element.value;
            }

            container.classList.remove('edit');

            textarea.disabled = false;
        }).catch(err => {
            textarea.disabled = false;
            console.error(err);
            alert(err);
        });
    });

    cancel.addEventListener('click', function (ev) {
        ev.preventDefault();
        ev.stopPropagation();

        container.classList.remove('edit');
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
