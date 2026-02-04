const { app, BrowserWindow, ipcMain, dialog } = require('electron');
const path = require('path');
const { chromium } = require('playwright-core');

let mainWindow;

function createWindow() {
    mainWindow = new BrowserWindow({
        width: 950,
        height: 750,
        webPreferences: {
            preload: path.join(__dirname, 'preload.js'),
            contextIsolation: true,
            nodeIntegration: false
        },
        autoHideMenuBar: true,
        resizable: true,
        backgroundColor: '#667eea',
        icon: path.join(__dirname, 'icon.png')
    });

    mainWindow.loadFile('index.html');
}

app.whenReady().then(createWindow);

app.on('window-all-closed', () => {
    if (process.platform !== 'darwin') {
        app.quit();
    }
});

app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
        createWindow();
    }
});

// Função para encontrar o Chrome/Edge instalado
function getChromeExecutablePath() {
    const fs = require('fs');
    const os = require('os');

    let possiblePaths = [];

    if (process.platform === 'win32') {
        possiblePaths = [
            'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
            'C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe',
            path.join(process.env.LOCALAPPDATA || '', 'Google\\Chrome\\Application\\chrome.exe'),
            'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe',
            'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe'
        ];
    } else if (process.platform === 'darwin') {
        possiblePaths = [
            '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
            '/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge'
        ];
    } else {
        possiblePaths = [
            '/usr/bin/google-chrome',
            '/usr/bin/chromium-browser',
            '/usr/bin/chromium',
            '/snap/bin/chromium'
        ];
    }

    for (const chromePath of possiblePaths) {
        if (fs.existsSync(chromePath)) {
            return chromePath;
        }
    }

    return null;
}

// Função principal de renovação
ipcMain.handle('renovar-livros', async (event, { login, senha }) => {
    let browser = null;
    let context = null;

    try {
        event.sender.send('progresso', '🔍 Procurando navegador...');

        const chromePath = getChromeExecutablePath();

        if (!chromePath) {
            throw new Error('Navegador não encontrado. Por favor, instale Google Chrome ou Microsoft Edge.');
        }

        event.sender.send('progresso', '🚀 Iniciando automação...');

        browser = await chromium.launch({
            headless: true,
            executablePath: chromePath,
            args: [
                '--no-sandbox',
                '--disable-setuid-sandbox',
                '--disable-dev-shm-usage',
                '--disable-blink-features=AutomationControlled'
            ]
        });

        context = await browser.newContext({
            viewport: { width: 1080, height: 1024 },
            userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36'
        });

        const page = await context.newPage();

        event.sender.send('progresso', '🌐 Acessando biblioteca Sophia...');

        await page.goto('https://biblioteca.sophia.com.br/5362/mobile/busca.asp?idioma=ptbr&acesso=web', {
            waitUntil: 'networkidle',
            timeout: 30000
        });

        event.sender.send('progresso', '📚 Navegando para renovações...');

        await page.waitForSelector('div.degrade.border-radius > a.icon-renovacao.ui-link', {
            timeout: 10000
        });
        await page.click('div.degrade.border-radius > a.icon-renovacao.ui-link');

        event.sender.send('progresso', '🔐 Autenticando usuário...');

        await page.waitForSelector('input#login', { timeout: 10000 });
        await page.fill('input#login', login);
        await page.fill('input#senha', senha);
        await page.click('a#botao_autenticar');

        event.sender.send('progresso', '⏳ Carregando seus livros...');

        // Esperar os livros aparecerem
        try {
            await page.waitForFunction(() => {
                return document.querySelectorAll('div.ui-content > ul > li').length > 0;
            }, { timeout: 15000 });
        } catch (error) {
            // Verificar se houve erro de login
            const loginError = await page.$('.erro, .mensagem-erro, [class*="erro"]');
            if (loginError) {
                throw new Error('Login ou senha incorretos. Verifique suas credenciais.');
            }
            throw new Error('Nenhum livro encontrado para renovação.');
        }

        // Extrair títulos dos livros
        const livros = await page.$$eval(
            'div.ui-content > ul > li > div.div-textoLista > h3',
            h3s => h3s.map(h3 => h3.textContent.trim())
        );

        if (livros.length === 0) {
            event.sender.send('progresso', 'ℹ️ Nenhum livro disponível para renovação');
            return { success: true, livros: [], mensagem: 'Nenhum livro para renovar' };
        }

        event.sender.send('progresso', `📖 Encontrados ${livros.length} livro(s)`);
        event.sender.send('livros-encontrados', livros);

        // Pegar os links de renovação
        const links = await page.$$('div.ui-content > ul > li > div.div-textoLista > div.ui-grid-a > div.ui-block-b > a');

        event.sender.send('progresso', '🔄 Iniciando renovações...');

        let renovados = 0;

        const delay = ms => new Promise(resolve => setTimeout(resolve, ms));

        for (let i = 0; i < links.length; i++) {
            const link = links[i];
            const href = await link.evaluate(el => el.href);

            if (href) {
                // Abrir link de renovação em nova aba
                await page.evaluate(url => {
                    window.open(url, '_blank');
                }, href);

                renovados++;
                event.sender.send('progresso', `✅ Renovado: ${livros[i] || `Livro ${i + 1}`} (${renovados}/${links.length})`);

                // Pequeno delay entre renovações
                await delay(2000);
            }
        }

        event.sender.send('progresso', `🎉 Renovação concluída! ${renovados} livro(s) renovado(s)`);

        return {
            success: true,
            livros: livros,
            totalRenovados: renovados
        };

    } catch (error) {
        console.error('Erro na renovação:', error);

        let mensagemErro = error.message;

        // Mensagens de erro
        if (mensagemErro.includes('timeout')) {
            mensagemErro = 'Tempo esgotado. Verifique sua conexão com a internet.';
        } else if (mensagemErro.includes('net::')) {
            mensagemErro = 'Erro de conexão. Verifique sua internet.';
        }

        event.sender.send('progresso', `❌ Erro: ${mensagemErro}`);

        return {
            success: false,
            error: mensagemErro
        };

    } finally {
        // Fechar navegador após 2 segundos
        if (browser) {
            setTimeout(async () => {
                try {
                    await browser.close();
                } catch (e) {
                    console.error('Erro ao fechar navegador:', e);
                }
            }, 2000);
        }
    }
});
