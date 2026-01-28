// Add CSRF token to all HTMX requests
document.body.addEventListener('htmx:configRequest', function(evt) {
    const csrfToken = document.querySelector('meta[name="csrf-token"]').getAttribute('content');
    if (csrfToken) {
        evt.detail.headers['X-CSRF-Token'] = csrfToken;
    }
});

document.body.addEventListener('htmx:afterRequest', function(evt) {
    const toast = document.getElementById('toast');
    if (!toast) return;
    
    const responseText = evt.detail.xhr.responseText;
    
    if (evt.detail.successful) {
        toast.innerText = responseText || 'Action successful';
        toast.className = 'success';
    } else {
        toast.innerText = 'Action failed: ' + (responseText || evt.detail.xhr.statusText);
        toast.className = 'error';
    }
    toast.style.display = 'block';
    setTimeout(() => { toast.style.display = 'none'; }, 5000);
});
