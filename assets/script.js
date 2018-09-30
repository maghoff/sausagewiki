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

const state = {
    hasBeenOpen: false,
    saving: false,
    editing: function () { return document.querySelector(".container").classList.contains('edit'); },
    hasCancelUrl: function () { return document.querySelector("a.button-cancel").getAttribute('href') !== ""; }
};

function openEditor() {
    const bodyElement = document.querySelector("body");
    const container = document.querySelector(".container");
    const rendered = container.querySelector(".rendered");
    const editor = container.querySelector(".editor");
    const textarea = editor.querySelector('textarea[name="body"]');
    const shadow = editor.querySelector('textarea.shadow-control');
    const form = document.getElementById('article-editor');
    const cancel = form.querySelector('.cancel');
    const cancelInteractionGroup = form.querySelector(".cancel-interaction-group");

    const footer = document.querySelector("footer");
    const lastUpdated = footer.querySelector(".last-updated");

    textarea.style.height = rendered.clientHeight + "px";

    container.classList.add('edit');
    updateFormEnabledState();

    autosizeTextarea(textarea, shadow);

    textarea.setSelectionRange(textarea.value.length, textarea.value.length);
    textarea.focus();

    if (state.hasBeenOpen) return;
    state.hasBeenOpen = true;

    textarea.addEventListener('input', () => autosizeTextarea(textarea, shadow));
    window.addEventListener('resize', () => autosizeTextarea(textarea, shadow));

    function updateFormEnabledState() {
        const baseEnabled = !state.saving && state.editing();
        const enabled = {
            cancel: baseEnabled && state.hasCancelUrl(),
        };

        cancelInteractionGroup.classList.remove(!enabled.cancel ? "interaction-group--root--enabled" : "interaction-group--root--disabled");
        cancelInteractionGroup.classList.add(enabled.cancel ? "interaction-group--root--enabled" : "interaction-group--root--disabled");

        for (const el of form.elements) {
            el.disabled = !baseEnabled;
        }

        // TODO: edit-link in footer?
    }

    function doSave() {
        state.saving = true;
        updateFormEnabledState();

        const body = queryArgsFromForm(form);

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
                        state.saving = false;
                        updateFormEnabledState();
                    });
            }

            if (!response.ok) throw new Error("Unexpected status code (" + response.status + ")");

            return response.json()
                .then(result => {
                    // Update url-bar, page title, footer and cancel link
                    const url = result.slug == "" ? "." : result.slug;
                    window.history.replaceState(null, result.title, url);
                    cancel.setAttribute("href", url);
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
                        document.activeElement && document.activeElement.blur();
                    }

                    state.saving = false;
                    updateFormEnabledState();
                    autosizeTextarea(textarea, shadow);

                    if (result.conflict) {
                        return alertAsync("Your edit came into conflict with another change " +
                            "and has not been saved.\n" +
                            "Please resolve the merge conflict and save again.");
                    }
                });
        }).catch(err => {
            state.saving = false;
            updateFormEnabledState();
            console.error(err);
            return alertAsync(err.toString());
        });
    }

    function doCancel() {
        Promise.resolve(!isEdited(form) || confirmDiscard())
            .then(doReset => {
                if (doReset) {
                    container.classList.remove('edit');
                    document.activeElement && document.activeElement.blur();
                    updateFormEnabledState();
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
            if (!state.editing()) return;

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
