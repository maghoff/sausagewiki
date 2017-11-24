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

function isEdited(form) {
    for (const {name, value, defaultValue} of form.elements) {
        if (name && (value !== defaultValue)) return true;
    }
    return false;
}

function loginDialog(loginUrl) {
    const loginDialogHtml = "<div class=popup><p>Your changes could not be saved</p><p>Log in and try again</p><div><button>Never mind</button> <a href='' target=login>Open login page</a></div></div>";

    const dialog = document.createElement("div");
    dialog.className = "modal-block";
    dialog.innerHTML = loginDialogHtml;

    const loginLink = dialog.querySelector("a");
    const dismiss = dialog.querySelector("button");

    loginLink.setAttribute("href", loginUrl);

    document.body.appendChild(dialog);
    loginLink.focus();

    function remove() {
        document.body.removeChild(dialog);
    }

    return new Promise((resolve, reject) => {
        loginLink.addEventListener("click", () => {
            remove();
            resolve();
        });

        dismiss.addEventListener("click", () => {
            remove();
            resolve();
        });
    });
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
        // TODO Disable other interaction as well: title editor, cancel and OK buttons

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
            if ((response.status === 401) && (response.headers.has("location"))) {
                return loginDialog(response.headers.get("location"))
                    .then(() => {
                        textarea.disabled = false;
                    });
            }

            if (!response.ok) throw new Error("Unexpected status code (" + response.status + ")");

            return response.json()
                .then(result => {
                    // Update url-bar, page title and footer
                    window.history.replaceState(null, result.title, result.slug == "" ? "." : result.slug);
                    document.querySelector("title").textContent = result.title;
                    lastUpdated.innerHTML = result.last_updated;
                    lastUpdated.classList.remove("missing");

                    // Update body:
                    rendered.innerHTML = result.rendered;

                    form.elements.title.value = result.title;
                    shadow.value = textarea.value = result.body;

                    // Update form:
                    form.elements.base_revision.value = result.revision;
                    for (const element of form.elements) {
                        element.defaultValue = element.value;
                    }

                    if (!result.conflict) {
                        container.classList.remove('edit');
                    }

                    textarea.disabled = false;
                    autosizeTextarea(textarea, shadow);

                    if (result.conflict) {
                        alert("Your edit came into conflict with another change and has not been saved.\n" +
                            "Please resolve the merge conflict and save again.");
                    }
                });
        }).catch(err => {
            textarea.disabled = false;
            console.error(err);
            alert(err);
        });
    });

    cancel.addEventListener('click', function (ev) {
        ev.preventDefault();
        ev.stopPropagation();

        if (!isEdited(form) || confirm("Discard changes?")) {
            container.classList.remove('edit');
            form.reset();
        }
    });

    window.addEventListener("beforeunload", function (ev) {
        if (isEdited(form)) {
            ev.preventDefault();
            return ev.returnValue = "Discard changes?";
        }
    });
}

document
    .getElementById("openEditor")
    .addEventListener("click", function (ev) {
        ev.preventDefault();
        ev.stopPropagation();

        openEditor();
    })

if (document.querySelector(".container").classList.contains("edit")) {
    openEditor();
}
