function debouncer(interval, callback) {
    let currentTimeout = null;

    function trigger() {
        currentTimeout = null;
        callback();
    }

    return function () {
        clearTimeout(currentTimeout);
        currentTimeout = setTimeout(trigger, interval);
    };
}

(function () {
    const form = document.querySelector('form.search');
    const input = form.querySelector('input');
    const results = form.querySelector('.live-results');
    const resultPrototype = document.getElementById('search-result-prototype').firstChild;

    form.addEventListener('submit', function (ev) {
        ev.preventDefault();
        ev.stopPropagation();
    });

    function submit() {
        if (input.value === "") {
            results.classList.remove("show");
            while (results.lastChild) results.removeChild(results.lastChild);
            return;
        }

        fetch(
            "_search?snippet_size=4&limit=3&q=" + encodeURIComponent(input.value),
            {
                headers: {
                    "Accept": "application/json",
                },
                credentials: "same-origin",
            }
        ).then(response => {
            if (!response.ok) throw new Error("Unexpected status code (" + response.status + ")");

            return response.json();
        }).then(result => {
            while (results.lastChild) results.removeChild(results.lastChild);

            result.hits.forEach(hit => {
                const item = resultPrototype.cloneNode(true);
                item.querySelector('.link').href = hit.slug || ".";
                item.querySelector('.title').textContent = hit.title;
                item.querySelector('.snippet').textContent = hit.snippet;
                results.appendChild(item);
            })
            results.classList.add("show");
        }).catch(err => {
            console.error(err);
            alert(err);
        });
    }
    const submitter = debouncer(200, submit);

    input.addEventListener('input', submitter);

    form.addEventListener('focusin', () => form.classList.add("focus"));
    form.addEventListener('focusout', function (ev) {
        for (let ancestor = ev.relatedTarget; ancestor; ancestor = ancestor.parentElement) {
            if (ancestor === form) return;
        }

        // We are now actually losing focus from the form:
        form.classList.remove("focus");
    });
})();
