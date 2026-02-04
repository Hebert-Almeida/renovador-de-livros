const btnRenovar = document.getElementById('btnRenovar');
const loginInput = document.getElementById('login');
const senhaInput = document.getElementById('senha');
const progressoContainer = document.getElementById('progressoContainer');
const progressoTexto = document.getElementById('progressoTexto');
const livrosContainer = document.getElementById('livrosContainer');
const listaLivros = document.getElementById('listaLivros');

// ==========================================
// SISTEMA DE TOAST (NOTIFICAÇÕES)
// ==========================================

function mostrarToast(mensagem, tipo = 'info') {
    const toast = document.getElementById('toast');

    // Definir ícones por tipo
    const icones = {
        'error': '❌',
        'success': '✅',
        'warning': '⚠️',
        'info': 'ℹ️'
    };

    const icone = icones[tipo] || icones['info'];
    toast.textContent = `${icone} ${mensagem}`;
    toast.className = 'toast';

    if (tipo === 'error') {
        toast.classList.add('error');
    } else if (tipo === 'success') {
        toast.classList.add('success');
    } else if (tipo === 'warning') {
        toast.classList.add('warning');
    }

    toast.classList.remove('hidden');

    // Remover após 4 segundos
    setTimeout(() => {
        toast.classList.add('hidden');
    }, 4000);
}

// ==========================================
// FUNÇÕES DE VALIDAÇÃO E RESET
// ==========================================

function resetarInterface() {
    btnRenovar.disabled = false;
    loginInput.disabled = false;
    senhaInput.disabled = false;
    btnRenovar.innerHTML = 'Renovar Livros';
}

function validarCampos(login, senha) {
    // Remover classe de erro anterior
    loginInput.classList.remove('input-error');
    senhaInput.classList.remove('input-error');

    if (!login && !senha) {
        loginInput.classList.add('input-error');
        senhaInput.classList.add('input-error');
        mostrarToast('Por favor, preencha login e senha!', 'warning');
        loginInput.focus();
        return false;
    }

    if (!login) {
        loginInput.classList.add('input-error');
        mostrarToast('Por favor, preencha o login!', 'warning');
        loginInput.focus();
        return false;
    }

    if (!senha) {
        senhaInput.classList.add('input-error');
        mostrarToast('Por favor, preencha a senha!', 'warning');
        senhaInput.focus();
        return false;
    }

    return true;
}

// ==========================================
// EVENTOS DE TECLADO
// ==========================================

// Enter no login vai para senha
loginInput.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
        senhaInput.focus();
    }
});

// Enter na senha inicia renovação
senhaInput.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
        btnRenovar.click();
    }
});

// Remover erro visual ao digitar
loginInput.addEventListener('input', () => {
    loginInput.classList.remove('input-error');
});

senhaInput.addEventListener('input', () => {
    senhaInput.classList.remove('input-error');
});

// ==========================================
// BOTÃO RENOVAR - EVENTO PRINCIPAL
// ==========================================

btnRenovar.addEventListener('click', async () => {
    const login = loginInput.value.trim();
    const senha = senhaInput.value.trim();

    // VALIDAÇÃO ANTES DE FAZER QUALQUER COISA
    if (!validarCampos(login, senha)) {
        return; // Para aqui se a validação falhar
    }

    // Desabilitar botão e inputs
    btnRenovar.disabled = true;
    loginInput.disabled = true;
    senhaInput.disabled = true;
    btnRenovar.innerHTML = '<span class="btn-icon spinner-small"></span> Processando...';

    // Mostrar progresso e limpar logs anteriores
    progressoContainer.classList.remove('hidden');
    progressoTexto.innerHTML = '';
    livrosContainer.classList.add('hidden');
    listaLivros.innerHTML = '';

    try {
        const resultado = await window.api.renovarLivros({ login, senha });

        if (resultado.success) {
            // Mensagem de sucesso
            if (resultado.totalRenovados > 0) {
                mostrarToast(
                    `Sucesso! ${resultado.totalRenovados} livro(s) renovado(s)!`,
                    'success'
                );
            } else {
                mostrarToast(
                    resultado.mensagem || 'Processo concluído',
                    'info'
                );
            }
        } else {
            // Mensagem de erro
            mostrarToast(resultado.error, 'error');
        }
    } catch (error) {
        console.error('Erro no processo:', error);
        mostrarToast(`Erro inesperado: ${error.message}`, 'error');
    } finally {
        // SEMPRE reabilitar, mesmo se der erro
        resetarInterface();
    }
});

// ==========================================
// RECEBER ATUALIZAÇÕES DO PROCESSO
// ==========================================

// Receber atualizações de progresso
window.api.onProgresso((mensagem) => {
    const p = document.createElement('p');
    p.className = 'log-item';
    p.textContent = mensagem;

    // Adicionar timestamp
    const timestamp = new Date().toLocaleTimeString('pt-BR');
    const span = document.createElement('span');
    span.className = 'timestamp';
    span.textContent = timestamp;
    p.prepend(span);

    progressoTexto.appendChild(p);

    // Auto-scroll para o final
    progressoTexto.scrollTop = progressoTexto.scrollHeight;

    // Animação de entrada
    setTimeout(() => p.classList.add('visible'), 10);
});

// Receber lista de livros
window.api.onLivrosEncontrados((livros) => {
    livrosContainer.classList.remove('hidden');

    livros.forEach((livro, index) => {
        setTimeout(() => {
            const li = document.createElement('li');
            li.className = 'livro-item';

            const numero = document.createElement('span');
            numero.className = 'livro-numero';
            numero.textContent = `${index + 1}.`;

            const titulo = document.createElement('span');
            titulo.className = 'livro-titulo';
            titulo.textContent = livro;

            li.appendChild(numero);
            li.appendChild(titulo);
            listaLivros.appendChild(li);

            // Animação
            setTimeout(() => li.classList.add('visible'), 10);
        }, index * 100);
    });
});