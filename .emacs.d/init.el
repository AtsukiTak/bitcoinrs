;;; *.~ とかのバックアップファイルを作らない
(setq make-backup-files nil)
;;; .#* とかのバックアップファイルを作らない
(setq auto-save-default nil)

;; cmdキーを superとして割り当てる
(setq mac-command-modifier 'super)

;; line numberの表示
(require 'linum)
(global-linum-mode 1)

;; tabサイズ
(setq default-tab-width 2)
(show-paren-mode 1)
;; save時にmode line を光らせる
(add-hook 'after-save-hook
      (lambda ()
        (let ((orig-fg (face-background 'mode-line)))
          (set-face-background 'mode-line "dark green")
          (run-with-idle-timer 0.1 nil
                   (lambda (fg) (set-face-background 'mode-line fg))
                   orig-fg))))

;; エラー音を鳴らなくする（多分みんなやってる）
(setq ring-bell-function 'ignore)
;; タイトルにフルパス表示
(setq frame-title-format "%f")

(setq custom-theme-directory "~/.emacs.d/themes/")
(load-theme 'molokai t)
(require 'package)
(add-to-list 'package-archives '("melpa" . "http://melpa.milkbox.net/packages/") t)
(add-to-list 'package-archives '("marmalade" . "http://marmalade-repo.org/packages/") t)
(package-initialize)
