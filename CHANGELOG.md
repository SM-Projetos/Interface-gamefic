# Changelog

## [0.1.0] - 2026-11-02

### Refinamentos visuais
- Aplicado degradê azul-marinho ao conteúdo, elevação dos cartões e destaque visual para seleção e hover.
- Ações superiores de fontes e atualizações agora abrem suas respectivas telas provisórias.

### Adicionado
- Base da aplicação desktop em Rust com Slint 1.18.1.
- Janela principal com sidebar fixa, navegação e barra superior.
- Tela Apps com categorias de Launchers e Emuladores.
- Cartões reutilizáveis com ícones SVG locais, descrição, status e realce de seleção.
- Grade flexível que ajusta a quantidade de colunas conforme a largura disponível.
- ScrollView para rolagem vertical do conteúdo.
- Alternância entre tema escuro e tema claro por paleta compartilhada.
- Páginas provisórias para os demais itens de navegação.
- Documentação de execução e limitações atuais no README.

### Limitações
- Estados de instalação exibidos nos cartões são demonstrativos.
- Instalação, remoção, atualização, detecção de aplicativos e integração com gerenciadores de pacotes ainda não foram implementadas.
