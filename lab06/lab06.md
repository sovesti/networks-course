# Практика 6. Транспортный уровень

## Wireshark: UDP (5 баллов)
Начните захват пакетов в приложении Wireshark и затем сделайте так, чтобы ваш хост отправил и
получил несколько UDP-пакетов (например, с помощью обращений DNS).
Выберите один из UDP-пакетов и разверните поля UDP в окне деталей заголовка пакета.
Ответьте на вопросы ниже, представив соответствующие скрины программы Wireshark.

#### Вопросы
1. Выберите один UDP-пакет. По этому пакету определите, сколько полей содержит UDP-заголовок.
   - <!-- todo -->
2. Определите длину (в байтах) для каждого поля UDP-заголовка, обращаясь к отображаемой
   информации о содержимом полей в данном пакете.
   - <!-- todo -->
3. Значение в поле Length (Длина) – это длина чего?
   - <!-- todo -->
4. Какое максимальное количество байт может быть включено в полезную нагрузку UDP-пакета?
   - <!-- todo -->
5. Чему равно максимально возможное значение номера порта отправителя?
   - <!-- todo -->
6. Какой номер протокола для протокола UDP? Дайте ответ и для шестнадцатеричной и
   десятеричной системы. Чтобы ответить на этот вопрос, вам необходимо заглянуть в поле
   Протокол в IP-дейтаграмме, содержащей UDP-сегмент.
   - <!-- todo -->
7. Проверьте UDP-пакет и ответный UDP-пакет, отправляемый вашим хостом. Определите
   отношение между номерами портов в двух пакетах.
   - <!-- todo -->

## Программирование. FTP

### FileZilla сервер и клиент (3 балла)
1. Установите сервер и клиент [FileZilla](https://filezilla.ru/get)
2. Создайте FTP сервер. Например, по адресу 127.0.0.1 и портом 21. 
   Укажите директорию по умолчанию для работы с файлами.
3. Создайте пользователя TestUser. Для простоты и удобства можете отключить использование сертификатов.
4. Запустите FileZilla клиента (GUI) и попробуйте поработать с файлами (создать папки,
добавить/удалить файлы).

Приложите скриншоты.

#### Скрины

Изначальное состояние:

<img width="2877" height="1677" alt="image" src="https://github.com/user-attachments/assets/b673a916-0836-4917-a54d-3c3d57026fbb" />

На сервере:

<img width="1568" height="1028" alt="image" src="https://github.com/user-attachments/assets/e44b0c8a-3a23-4734-98b2-9e4ce80d6680" />

Создание папки на сервере с клиента:

<img width="2879" height="1679" alt="image" src="https://github.com/user-attachments/assets/6f8267b1-df14-4c4f-aa23-223b82a5f871" />

Передача файла:

<img width="2879" height="1681" alt="image" src="https://github.com/user-attachments/assets/af88def2-8034-4b69-8665-ec3ae9abbd04" />

Удаление файла:

<img width="2879" height="1626" alt="image" src="https://github.com/user-attachments/assets/1d13fe5e-397a-40d1-bf82-94576deba25b" />


### FTP клиент (3 балла)
Создайте консольное приложение FTP клиента для работы с файлами по FTP. Приложение может
обращаться к FTP серверу, созданному в предыдущем задании, либо к какому-либо другому серверу 
(есть много публичных ftp-серверов для тестирования, [вот](https://dlptest.com/ftp-test/) один из них).

Приложение должно:
- Получать список всех директорий и файлов сервера и выводить его на консоль
- Загружать новый файл на сервер
- Загружать файл с сервера и сохранять его локально

Бонус: Не используйте готовые библиотеки для работы с FTP (например, ftplib для Python), а реализуйте решение на сокетах **(+3 балла)**.

#### Демонстрация работы

Клиент написан на Rust (2024 Edition) с использованием высокоуровневой библиотеки для работы с FTP ```suppaftp```. Чтобы его собирать, нужен [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html). Запуск клиента:

```
$ git checkout f0b446 # коммит без UI: https://github.com/sovesti/networks-course/commit/f0b446a7930afb6a99d51d9a4abae7afb5332f3d
$ cd ftp-client
$ cargo run -- --address ftp.dlptest.com:21 --user dlpuser --password rNrKYTX9g7z3RgJRmxWuGHbeu list  # после -- идут аргументы, передаваемые клиенту
```

или

```
$ cd ftp-client
$ cargo build
...Finished `dev` profile
$ target/debug/ftp-client.exe list # запуск со стандартными аргументами: сервер на 127.0.0.1:21, пользователь TestUser, пароль 12345678
```

Структура файлов на сервере:

<img width="451" height="223" alt="image" src="https://github.com/user-attachments/assets/e3f57276-8d04-4e4a-bd74-c05e7abfe704" />

Список файлов:

```
$ target\debug\ftp-client.exe list
drwxrwxrwx 1 ftp ftp               0 Apr 12 21:37 dir-on-server
-rw-rw-rw- 1 ftp ftp              17 Apr 12 21:28 hello.txt
Operation completed succesfully
```

Загрузка файла с сервера:

```
$ target\debug\ftp-client.exe download hello.txt
Received 17 bytes
Operation completed succesfully
$ cat hello.txt
hello from ftp!
```

Загрузка файла на сервер:

```
target\debug\ftp-client.exe upload kotenok.png
Sent 1385925 bytes
Operation completed succesfully
```

<img width="2290" height="1307" alt="image" src="https://github.com/user-attachments/assets/b4136f1a-4f0b-41d8-ac1c-2eaa4da79d6b" />

### GUI FTP клиент (4 балла)
Реализуйте приложение FTP клиента с графическим интерфейсом. НЕ используйте C#.

Возможный интерфейс:

<img src="images/example-ftp-gui.png" width=300 />

В приложении должна быть поддержана следующая функциональность:
- Выбор сервера с указанием порта, логин и пароль пользователя и возможность
подключиться к серверу. При подключении на экран выводится список всех доступных
файлов и директорий
- Поддержаны CRUD операции для работы с файлами. Имя файла можно задавать из
интерфейса. При создании нового файла или обновлении старого должно открываться
окно, в котором можно редактировать содержимое файла. При команде Retrieve
содержимое файла можно выводить в главном окне.

#### Демонстрация работы

Приложение реализовано на фреймворке Dioxus, для сборки понадобятся Rust'овая цель ```wasm32-unknown-unknown``` и консольное приложение ```dioxus-cli```, инструкцию по установке можно найти в документации фреймворка: https://dioxuslabs.com/learn/0.7/getting_started/. Тестировалось только под Windows. Запуск:

```
$ cd ftp-client
$ dx serve --desktop
```

Или в браузере:

```
$ cd ftp-client
$ dx serve --web
```

Начальное состояние:

<img width="310" height="471" alt="image" src="https://github.com/user-attachments/assets/33f72803-7537-42ca-8835-88431f439030" />

Неверный пароль:

<img width="310" height="471" alt="image" src="https://github.com/user-attachments/assets/25c9f052-c9ff-4066-8537-cf022d857577" />

Успешное подключение:

<img width="310" height="471" alt="image" src="https://github.com/user-attachments/assets/d88efb37-f315-496e-abf8-d5b1674482a2" />

Загрузка файла:

<img width="310" height="471" alt="image" src="https://github.com/user-attachments/assets/a8479887-efd7-401c-bbd7-cd588bf53a87" />

<img width="310" height="471" alt="image" src="https://github.com/user-attachments/assets/a05c55ca-93eb-4cb2-8991-173074c572ef" />

<img width="310" height="471" alt="image" src="https://github.com/user-attachments/assets/78cf2780-57ae-4421-9279-3c11c5e32aca" />

<img width="1016" height="297" alt="image" src="https://github.com/user-attachments/assets/c6fb5d7d-06a4-4384-8ead-4681a85f16be" />

Редактирование файла:

<img width="310" height="471" alt="image" src="https://github.com/user-attachments/assets/db52648e-99b0-4565-95f7-d759c4a2538d" />

<img width="310" height="471" alt="image" src="https://github.com/user-attachments/assets/16a8b7f6-2286-43ef-8f68-78509c3b17b8" />

Скачивание файла (буквы ```ААААAAA``` добавлены на предыдущем шаге). При скачивании файл не выводится на экран, а сохраняется в папку, из которой запущен клиент:

<img width="375" height="570" alt="image" src="https://github.com/user-attachments/assets/1d7f6470-7bc5-4584-ae4c-4629af794031" />

Удаление файла:

<img width="310" height="471" alt="image" src="https://github.com/user-attachments/assets/e4d8635f-b404-42ef-83ab-80ca37096219" />

### FTP сервер (5 баллов)
Реализуйте свой FTP сервер, который работает поверх TCP сокетов. Вы можете использовать FTP клиента, реализованного на прошлом этапе, для тестирования своего сервера.
Сервер должен реализовать возможность авторизации (с указанием логина/пароля) и поддерживать команды:
- CWD
- PWD
- PORT
- NLST
- RETR
- STOR
- QUIT

#### Демонстрация работы
todo
