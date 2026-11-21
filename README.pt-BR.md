<p align="center"><img src="assets/logo.svg" alt="WinRust" width="160"></p>

# WinRust

[English](README.md) | [Русский](README.ru.md) | **Português (Brasil)**

O WinRust reúne programas nativos em Rust que substituem a barra de tarefas, o Explorador de Arquivos, a busca e o Gerenciador de Tarefas do Windows. Inclui também ferramentas para limpar e analisar discos e visualizar fotos. O instalador permite adicionar e remover os componentes.

<p align="center"><img src="assets/th.jpg" alt="WinRust" width="480"></p>

> O WinRust pode desativar a Pesquisa do Windows, ocultar a barra de tarefas e substituir a interface do Explorador de Arquivos. O instalador não tem assinatura digital e pode acionar um aviso do SmartScreen. Antes de instalar, ele cria um ponto de restauração. Prefira testar primeiro em uma máquina de reserva ou em uma máquina virtual.

## Instalação

1. Baixe o instalador para Windows 10 ou 11 de 64 bits. As versões oficiais estão na [página de lançamentos](https://github.com/Lemxy/WinRust/releases).
2. Execute o instalador como administrador.
3. Selecione os componentes e clique no botão de instalação. Em uma instalação nova, marque **rust-explorer** se quiser substituir o Explorador de Arquivos do Windows. A mesma opção fica na engrenagem da linha do **topbar**. O instalador respeita a ordem das dependências e informa quais componentes falharam ou foram ignorados.

Se o serviço de Pesquisa do Windows estiver ausente, o instalador ignora essa etapa e exibe um aviso.

Se o UAC pedir a senha de outra conta administrativa, as tarefas agendadas, o perfil do PowerShell e as permissões de busca serão configurados para a conta conectada. Se o instalador não puder reconhecer essa conta, a operação é interrompida antes de modificar o sistema.

Para atualizar, execute o instalador mais recente e selecione os componentes desejados. O WinRust conserva suas preferências e os valores originais do Windows que foram salvos para restauração.

O idioma padrão é **Português (Brasil)**. Para selecionar outro idioma, use o botão no canto superior direito do instalador ou abra **Idioma do WinRust** no menu de contexto da barra. Reabra as janelas do WinRust para aplicar a preferência.

## Remoção

- **Remover tudo** no instalador restaura a Pesquisa do Windows, a barra de tarefas, o Explorador de Arquivos e as configurações de serviços salvas.
- Para remover um componente individualmente, use `plugin-launcher uninstall <componente>`. Consulte [Linha de comando](#linha-de-comando).
- Se a área de trabalho ou a barra desaparecer, pressione **Ctrl+Alt+Del**, abra o **Gerenciador de Tarefas**, escolha **Executar nova tarefa** e digite `explorer.exe`. Depois execute o instalador e selecione **Remover tudo**.

Ao desinstalar o `rust-explorer`, o WinRust recupera os tipos de inicialização de serviços e o limite de agrupamento de processos `svchost` salvos na instalação. Instalações antigas sem essa cópia conservam os valores atuais e exibem um aviso.

## Componentes

| Componente | Função e alterações no Windows |
|---|---|
| **rust-search** | Pesquisa arquivos de unidades NTFS com Alt+Espaço. Desativa o serviço de Pesquisa do Windows, bloqueia o `SearchHost.exe` e oculta a caixa de pesquisa da barra padrão. |
| **rtaskmgr-win32** | Gerenciador de Tarefas aberto por Ctrl+Shift+Esc ou pelo comando `taskmgr.exe`. A instalação preserva o arquivo de sistema e inclui o limpador **rs-du**. A página **Desempenho** oferece ajustes reversíveis, como perfis Jogo e Trabalho, limpeza do cache de memória, bloqueio de processos auxiliares da interface, atualização semanal com `winget`, exclusões do Defender e ajustes de NTFS e do arquivo `hosts`. As páginas **Inicialização** e **O que está lento** mostram programas de inicialização, tempo de drivers e interrupções, janelas travadas, disco e tempo de inicialização do Windows. |
| **topbar** | Barra de tarefas no estilo Windows 11 com janelas, relógio, volume e menus. Inclui **reshell**, **rs-du** e configurações opcionais no perfil do PowerShell para `zoxide`, `rg`, `eza` e `bat`. Use Win+1/2/3 para alternar áreas de trabalho, Win+D para mostrar a área de trabalho, Win+I para abrir Configurações e Win+E para abrir o Explorador. A engrenagem permite posicionar a barra embaixo ou nas laterais e alinhar os botões à esquerda. |
| **rust-explorer** | Explorador de Arquivos com abas e área de trabalho própria. Requer o topbar. Permite cópia de arquivos com progresso, compactação em ZIP e extração de ZIP e 7z. O item **Mostrar na pasta** seleciona o arquivo nas pastas comuns. Botões de voltar e avançar do mouse funcionam. O menu da área de trabalho permite ocultar os ícones. Na instalação inicial, pode ajustar PcaSvc, InventorySvc, lfsvc, cbdhsvc, DoSvc e o agrupamento de processos svchost quando consegue guardar os valores anteriores. |
| **wiztree** | Mapa do espaço utilizado no disco, acessível pelo atalho **WinRust Disk**. |
| **photoview** | Visualizador de fotos com navegação por setas e rotação EXIF. Aparece em **Abrir com**. Você pode escolher o aplicativo padrão nas Configurações do Windows. |
| **rust-settings** | Configurações de tela, som, energia, armazenamento, rede, Bluetooth, aplicativos, Windows Update, tema e sistema. Inclui organização de monitores por arraste, Wi-Fi 802.1X, proxy, VPN e opções por aplicativo. Win+I abre o painel. Algumas páginas ainda são fornecidas pelo Windows. |
| **debloat** | Ajustes reversíveis em telemetria, Copilot, Recall, anúncios do menu Iniciar e atividade do Edge em segundo plano. Use `install debloat --yes` na linha de comando para instalar. |
| **purge** | Remove aplicativos pré-instalados e tarefas de telemetria. A exclusão é **irreversível**. O comando `purge.exe` só lista. A remoção pode ser acionada com `purge.exe --apply`. As opções `--onedrive` e `--edge` incluem esses dois programas. |
| **chrome-rs** | Navegador experimental, ainda fora do instalador. |

## Estilos

A engrenagem da barra abre o menu de estilos. A escolha se aplica ao Explorador e à busca.

| Estilo | Aparência |
|---|---|
| **Normal** | Azul do Windows 11 com a barra embaixo. |
| **Terminal** | Verde sobre preto, Consolas e cantos retos. |
| **Barra flutuante** | Barra central arredondada. Janelas maximizadas podem usar toda a tela. |
| **Janelas lado a lado** | Barra fina no topo com destaque âmbar e opção de organizar janelas abertas. |
| **Estrada branca** | Fundo marfim com triângulos discretos e traços dourados. |
| **Rubi** | Fundo grená com destaque rubi e brilho suave. |
| **Moderno** | Barra com Iniciar antes da busca, ícones sobre bases de contraste e Alt+Tab com lista de janelas e prévia ao vivo. O Explorador e as Configurações do WinRust seguem a paleta, que acompanha a preferência de tema claro ou escuro dos aplicativos no Windows. |

A engrenagem também controla o ocultamento automático e o posicionamento lateral. O estilo é salvo em `%ProgramData%\WinRust\style.txt`.

## Medições de desempenho

O projeto mediu uma máquina com Windows 11 IoT Enterprise 10.0.26300, RTX 4060 Laptop e NTFS. Os valores são referências aproximadas.

| Operação | Windows | WinRust |
|---|---:|---:|
| Buscar arquivos com `dir /s` ou rust-search | 8.340 ms | 45 ms |
| Buscar arquivos com `Get-ChildItem -Recurse` ou rust-search | 16.200 a 28.600 ms | 45 ms |
| Abrir uma pasta (mediana de cinco execuções) | 420 ms | 60 ms |
| Memória da interface com topbar, rust-explorer, rs-shell e reshell | 366 MB | 46 MB |
| Memória incluindo o índice de busca | 366 MB | 99 MB |
| Tamanho do executável do Explorador | 3.280 KB | 1.179 KB |
| Tamanho do executável do Gerenciador de Tarefas | 5.791 KB | 954 KB |
| CPU da barra em repouso (% de um núcleo) | 0,00 a 0,10% | 0,05 a 0,10% |
| Copiar 5.000 arquivos pequenos (cerca de 160 MB) | 16,3 s | 2,3 s |
| Copiar um arquivo de 1 GB | 0,58 s | 0,60 s |
| Compactar 5.000 arquivos em ZIP | 48,4 s | 0,75 s |

O rust-search usa um índice MFT de 53 MB em memória, enquanto `dir` e PowerShell percorrem o disco. O Explorer ainda estava em execução na medição de memória. A comparação de busca não inclui a Pesquisa do Windows. Os testes de cópia e ZIP usaram cache de memória em ambos os casos. O ZIP do WinRust ocupou 0,9 MB e o do PowerShell, 0,7 MB.

## Capturas de tela

![Barra de tarefas na área de trabalho](docs/screenshots/taskbar.webp)
![Menu da barra de tarefas](docs/screenshots/taskbar-menu.png)
![Novo visual do menu da barra](docs/screenshots/topbar-menu-new.png)
![Gerenciador de Tarefas com interface em russo](docs/screenshots/task-manager.webp)
![Explorador de Arquivos](docs/screenshots/explorer.png)

## Desenvolvimento

| Caminho | Conteúdo |
|---|---|
| `src/` | Instalador e lista de componentes. `install/` reúne as operações de instalação e remoção. A interface e a linha de comando chamam as mesmas operações. |
| `common/` | Idiomas, estilos, processamento da saída de `reg.exe` e leitor MFT. As mensagens fixas em português ficam em `common/src/pt_br.rs`. Os textos formatados ficam junto das chamadas a `tf!` e `pf!`. |
| `plugins/` | Código dos componentes. `rust-search` e `chrome-rs` são compilados separadamente para usar `panic = "unwind"`. |
| `ReShell/` | Atalhos de área de trabalho e recuperação da barra. |
| `payload.txt` e `payload/` | Lista dos executáveis incluídos no instalador e pasta de preparação da compilação. |
| `scripts/` | Compilação, atualização de componentes, testes de desempenho e publicação. |

### Compilação

Use Windows x64 e Rust estável com a cadeia de ferramentas MSVC. Execute o script a partir da raiz do repositório.

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-all.ps1
```

O instalador com os componentes compilados fica em `target\release\plugin-launcher.exe`. Feche qualquer instância do instalador antes de recompilar. Os testes são `cargo test --workspace` na raiz e `cargo test` em `plugins\rust-search`.

| Script | Uso |
|---|---|
| `build-all.ps1` | Compila os componentes, preenche `payload/` e compila o instalador. |
| `build-plugins.ps1` | Compila os componentes e atualiza `payload/`. |
| `update-plugin.ps1 <componente>` | Recompila e reinstala um componente. Solicita elevação pelo UAC. |
| `user-mode.ps1` e `dev-update.ps1` | Executam e atualizam topbar, rust-explorer, rs-shell e reshell em modo de desenvolvimento. |
| `bench.ps1` | Executa medições de busca e do Gerenciador de Tarefas. |
| `publish.ps1` e `release.ps1` | Enviam alterações e publicam o instalador no GitHub. |

### Linha de comando

Use um terminal aberto como administrador.

```text
plugin-launcher list
plugin-launcher install <componente>... --yes
plugin-launcher update <componente>
plugin-launcher uninstall <componente>
plugin-launcher rollback --yes
```

Use `install all --yes` para instalar os componentes padrão ou `install all rust-explorer --yes` para incluir o Explorador. A opção `--yes` confirma a instalação. Sem ela, o instalador lista os avisos. O retorno é 0 em caso de sucesso, 1 se algum componente falhar ou 2 quando os argumentos estiverem inválidos. Sem argumentos, o instalador abre a interface. O log fica em `%ProgramData%\plugin-launcher\install.log`.

### Como adicionar um componente

1. Crie uma crate em `plugins/<nome>` e adicione-a ao workspace no `Cargo.toml` da raiz.
2. Adicione o executável ao `payload.txt`.
3. Implemente a instalação e remoção em `src/install/<nome>.rs` e declare o módulo em `src/install/mod.rs`. Guarde os valores originais antes de modificar o Windows.
4. Adicione uma entrada à lista `PLUGINS` em `src/main.rs`.

## Licença

Você pode escolher entre a licença [MIT](LICENSE-MIT) e a [Apache 2.0](LICENSE-APACHE).
