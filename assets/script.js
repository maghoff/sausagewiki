"use strict";

function autosizeTextarea(textarea, shadow) {
    shadow.style.width = textarea.clientWidth + "px";
    shadow.value = textarea.value;
    textarea.style.height = shadow.scrollHeight + "px";
}

function queryArgsFromForm(form) {
    const items = [];
    for (const {name, value, type, checked} of form.elements) {
        if (!name) continue;
        if (type === "radio" && !checked) continue;
        items.push(encodeURIComponent(name) + '=' + encodeURIComponent(value));
    }
    return items.join('&');
}

function isEdited(form) {
    for (const {name, value, defaultValue, checked, defaultChecked} of form.elements) {
        if (name && ((value !== defaultValue) || (checked !== defaultChecked))) return true;
    }
    return false;
}

function instantiate(templateId) {
    return document.getElementById(templateId).firstElementChild.cloneNode(true);
}

function popup(dialog) {
    document.body.appendChild(dialog);
    dialog.querySelector(".primary").focus();

    return new Promise((resolve, reject) => {
        function handler(ev) {
            document.body.removeChild(dialog);
            resolve(ev.target.getAttribute("data-value"));
        }

        const buttons = dialog.querySelectorAll('.btn-row>*');
        for (let i = 0; i < buttons.length; ++i)
            buttons[i].addEventListener("click", handler);
    });
}

function loginDialog(loginUrl) {
    const dialog = instantiate("login");
    dialog.querySelector("a").setAttribute("href", loginUrl);
    return popup(dialog);
}

function alertAsync(message) {
    const dialog = instantiate("alert");
    dialog.querySelector(".message").textContent = message;
    return popup(dialog);
}

function confirmDiscard() {
    return popup(instantiate("confirm-discard"));
}

let hasBeenOpen = false;
function openEditor() {
    const bodyElement = document.querySelector("body");
    const container = document.querySelector(".container");
    const rendered = container.querySelector(".rendered");
    const editor = container.querySelector(".editor");
    const textarea = editor.querySelector('textarea[name="body"]');
    const shadow = editor.querySelector('textarea.shadow-control');
    const form = document.getElementById('article-editor');
    const cancel = form.querySelector('.cancel');

    const footer = document.querySelector("footer");
    const lastUpdated = footer.querySelector(".last-updated");

    textarea.style.height = rendered.clientHeight + "px";

    container.classList.add('edit');

    autosizeTextarea(textarea, shadow);

    textarea.setSelectionRange(textarea.value.length, textarea.value.length);
    textarea.focus();

    if (hasBeenOpen) return;
    hasBeenOpen = true;

    textarea.addEventListener('input', () => autosizeTextarea(textarea, shadow));
    window.addEventListener('resize', () => autosizeTextarea(textarea, shadow));

    function doSave() {
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
            // I don't know how to more precisely determine that we hit a login redirect:
            const probablyLoginRedirect = response.redirected &&
                (response.headers.get("content-type") !== "application/json");

            if (probablyLoginRedirect) {
                return loginDialog(response.url)
                    .then(() => {
                        textarea.disabled = false;
                    });
            }

            if (!response.ok) throw new Error("Unexpected status code (" + response.status + ")");

            return response.json()
                .then(result => {
                    // Update url-bar, page title and footer
                    window.history.replaceState(null, result.title, result.slug == "" ? "." : result.slug);
                    // TODO Cancel-link URL should be updated to new slug
                    document.querySelector("title").textContent = result.title;
                    lastUpdated.innerHTML = result.last_updated;
                    lastUpdated.classList.remove("missing");

                    // Update body:
                    rendered.innerHTML = result.rendered;

                    form.elements.title.value = result.title;
                    shadow.value = textarea.value = result.body;

                    form.querySelector(`.theme-picker--option[value=${JSON.stringify(result.theme)}]`).checked = true;
                    bodyElement.className = `theme-${result.theme}`;

                    // Update form:
                    form.elements.base_revision.value = result.revision;
                    for (const element of form.elements) {
                        element.defaultValue = element.value;
                        element.defaultChecked = element.checked;
                    }

                    if (!result.conflict) {
                        container.classList.remove('edit');
                    }

                    textarea.disabled = false;
                    autosizeTextarea(textarea, shadow);

                    if (result.conflict) {
                        return alertAsync("Your edit came into conflict with another change " +
                            "and has not been saved.\n" +
                            "Please resolve the merge conflict and save again.");
                    }
                });
        }).catch(err => {
            textarea.disabled = false;
            console.error(err);
            return alertAsync(err.toString());
        });
    }

    function doCancel() {
        Promise.resolve(!isEdited(form) || confirmDiscard())
            .then(doReset => {
                if (doReset) {
                    container.classList.remove('edit');
                    form.reset();

                    let selectedTheme = form.querySelector(`.theme-picker--option[checked]`).value;
                    bodyElement.className = `theme-${selectedTheme}`;
                }
            });
    }

    form.addEventListener("submit", function (ev) {
        ev.preventDefault();
        ev.stopPropagation();
        doSave();
    });

    cancel.addEventListener('click', function (ev) {
        ev.preventDefault();
        ev.stopPropagation();
        doCancel();
    });

    window.addEventListener("beforeunload", function (ev) {
        if (isEdited(form)) {
            ev.preventDefault();
            return ev.returnValue = "Discard changes?";
        }
    });

    document.addEventListener("keypress", function (ev) {
        const accel = ev.ctrlKey || ev.metaKey; // Imprecise, but works cross platform
        if (ev.key === "Enter" && accel) {
            const isEditing = container.classList.contains('edit');
            if (!isEditing) return;

            ev.stopPropagation();
            ev.preventDefault();

            doSave();
        }
    });

    const themeOptions = form.querySelectorAll(".theme-picker--option");
    for (let themeOption of themeOptions) {
        themeOption.addEventListener("click", function (ev) {
            bodyElement.className = `theme-${ev.target.value}`;
        });
    }
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
