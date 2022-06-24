# Rusty Bash

## コレは何？

1. もともと上田がRustの練習にbashを作り始めた
2. きれいに書けば「コードレベルでカスタマイズできるbashになるんじゃないか」と色気が出た
3. ということで開発中


## 開発への参加方法

とりあえずルールがまだないけど、がんばって対応します。

## 目指すこと

* 軽快さ
* 堅牢さ
* コードの読みやすさ、手の入れやすさ
* 端末で使うシェルとして普段使いできるようにする（これはそんなに遠い未来にはならない模様）

## 限界

* Rustはじめてさわったのが2022年1月という初心者がやっている。Rustなんもわからん。たすけて。
* バイナリのサイズは大きくなる見通しなので、シェルスクリプトでループをぶん回す用途には向かない。
* 今のところ、本業で疲れたときの逃避行動でコードを書いているので、コード内にコメントがないです。

## シェルとしての機能

だんだんbashに近づいてくるはず。現在のインタプリタとしての機能は、[test/test.bash](https://github.com/ryuichiueda/rusty_bash/blob/main/test/test.bash)を見ると分かる。


### bashにない独自オプション

* -d: output debug information

## どんなふうにbashを実装しているか

* bashの各要素を表す構造体と評価方法、パース方法を`elems_hoge.rs`に実装
    * パースにはパーサコンビネータを自分でゴリゴリ書いている。
* なにか抽象化しているようなものは`abst_hoge.rs`に実装しているが議論の余地あり
* 端末とのやりとりを`term.rs`に実装（かなり素朴で無駄の多い実装。誰か助けて！）

## ライセンス

* 3条項BSD
* 各外部クレートのライセンス等は、いまのところとりまとめてません。
