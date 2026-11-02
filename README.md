# Interface-gamefic

Interface desktop do Gamefic, desenvolvida em **Rust** e **Slint**.

## Requisitos

- Rust e Cargo (toolchain estável compatível com Slint 1.18).
- Dependências nativas do backend gráfico do Slint para sua distribuição Linux.

## Executar

```bash
cargo run
```

## Estrutura

- `src/main.rs`: inicialização da aplicação.
- `build.rs`: compilação declarativa da interface Slint.
- `ui/main.slint`: janela, navegação, temas, catálogo e componentes reutilizáveis.
- `assets/icons/`: ícones vetoriais locais dos aplicativos e do Gamefic.

## Interface

A tela de Apps segue a referência visual fornecida: sidebar fixa, barra superior, conteúdo rolável, categorias em grade flexível e cartões de dimensões consistentes. O tema escuro é o padrão; o botão de configurações alterna entre tema escuro e claro.

A grade usa cartões com largura preferencial de 323 px, altura de 212 px e espaçamento de 20 px. A disposição quebra automaticamente para novas linhas quando a janela é reduzida.

## Estado atual

Esta versão entrega a estrutura visual e interações locais de navegação, seleção de cartão e alternância de tema. Os indicadores de instalação são dados demonstrativos: instalação, remoção, atualização, detecção de pacotes e integração com o sistema operacional ainda não estão conectadas.
