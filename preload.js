const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('api', {
    renovarLivros: (credenciais) => ipcRenderer.invoke('renovar-livros', credenciais),
    onProgresso: (callback) => ipcRenderer.on('progresso', (event, mensagem) => callback(mensagem)),
    onLivrosEncontrados: (callback) => ipcRenderer.on('livros-encontrados', (event, livros) => callback(livros))
});