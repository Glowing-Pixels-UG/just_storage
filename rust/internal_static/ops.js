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
