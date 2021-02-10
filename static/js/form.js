'use strict';

function show(ele) {
  ele.style.display = '';
}

function hide(ele) {
  ele.style.display = 'none';
}

function fill_paste_name_when_choosing_file(e) {
  const tab_root = e.closest('.tab-pane');
  const name = tab_root.querySelector('.name');
  console.log(`root ${tab_root}, name ${name}`);
  const files = e.files;
  if (files.length > 0) {
    name.value = files[0].name;
  }
}

const success_card_class = 'card text-white bg-success mb-3 status-card';
const working_card_class = 'card text-white bg-secondary mb-3 status-card';
const fail_card_class = 'card text-black bg-warning mb-3 status-card';

function update_card(card, classes, state, msg) {
  // UI Elements
  const state_dom = card.querySelector('.state');
  const msg_dom = card.querySelector('.msg');

  card.classList = classes;
  state_dom.innerHTML = state;
  msg_dom.innerHTML = msg;
  show(card);
}

function get_pastes() {
  // Get current list from localStorage
  const s = window.localStorage;
  let list_json = s.getItem('pastes');
  if (list_json === null) {
    let array = [];
    return array;
  } else {
    let array = JSON.parse(list_json);
    return array;
  }
}

function add_paste_to_storage(paste) {
  const s = window.localStorage;

  let array = get_pastes();
  // Modify Map
  paste.create_time = Date.now();
  array.push(paste);

  // Write back
  const new_json = JSON.stringify(array);
  s.setItem('pastes', new_json);
}

function expire_paste_in_storage() {
  const array = get_pastes();
  const result = array.filter((paste) => {
    if (paste.expire_time === null) {
      return true;
    }
    let expire_time = new Date(paste.expire_time);
    if (expire_time.getTime() > Date.now()) {
      return true;
    } else {
      return false;
    }
  });
  window.localStorage.setItem('pastes', JSON.stringify(result));
}

function delete_paste_in_storage(id) {
  const array = get_pastes();
  const result = array.filter((paste) => {
    if (paste.id != id) {
      return true;
    } else {
      return false;
    }
  });
  window.localStorage.setItem('pastes', JSON.stringify(result));
}

function show_pastes_from_storage(e) {
  expire_paste_in_storage();
  // UI Elements
  const list = e.querySelector('.local-paste-list');
  // Stuff
  const array = get_pastes();
  if (array.size == 0) {
    // Do nothing
    return;
  }

  list.innerHTML = '';
  for (const paste of array) {
    let a = document.createElement('a');
    let create_time = new Date(paste.create_time);
    a.innerText = paste.id + " created on " + create_time.toLocaleString();
    a.href = '#';
    a.classList = 'list-group-item list-group-item-action';
    a.onclick = () => { fill_id_and_key(e, paste.id, paste.key); };
    a.dataset.id = paste.id;
    a.dataset.key = paste.key;
    list.appendChild(a);
  }
}

function fill_id_and_key(e, id, key) {
  e.querySelector('.id').value = id;
  if (e.querySelector('.key') != undefined) {
    e.querySelector('.key').value = key;
  }
}

function update_paste_list() {
  show_pastes_from_storage(document.getElementById('modify'));
  show_pastes_from_storage(document.getElementById('view'));
  show_pastes_from_storage(document.getElementById('delete'));
}

function create() {
  // UI Elements
  const create = document.getElementById('create');
  const text_input = create.querySelector('.text-input');
  const create_status = create.querySelector('.status-card');
  const file_selector = create.querySelector('.paste-from-file');

  update_card(create_status, working_card_class, "Uploading", "Hang tight...");

  const val = text_input.value;
  const name = create.querySelector('.name').value;
  const exp_time = create.querySelector('.expire-in').value;
  const exp_time_unit = create.querySelector('.expire-time-unit').value;

  // Initate form data
  const formData = new FormData();
  if (file_selector.files.length > 0) {
    formData.append('content', file_selector.files[0]);
  } else {
    if (val.length == 0) {
      create_status.classList = fail_card_class;
      state.innerHTML = "Warning";
      msg.innerHTML = "Cannot create empty paste.";
      return;
    }
    formData.append('c', val);
  }

  if (exp_time.length != 0) {
    formData.append('expire_after', exp_time * exp_time_unit);
  }

  if (name.length != 0) {
    formData.append('name', name);
  }

  const req = new Request('/', {mode: 'cors', method: 'POST', body: formData});
  
  fetch(req)
    .then(res => res.json())
    .then(res => {
      if (res.success) {
        let result_msg = `Paste ID: <a href=${res.info.id}>${res.info.id}</a>, modify key: ${res.info.key}. `;
        let t = null;
        if (res.info.expire_time != null) {
          t = new Date(res.info.expire_time);
          result_msg += `Paste will expire at ${t.toLocaleString()}.`;
        }
        update_card(create_status, success_card_class, "Success", result_msg);
        add_paste_to_storage({
          id: res.info.id,
          key: res.info.key,
          expire_time: t,
        });
        update_paste_list();
      } else {
        update_card(create_status, fail_card_class, "Failed", res.message);
      }
    })
    .catch(err => {
      update_card(create_status, fail_card_class, "Failed", err.message);
    });
}

function view() {
  const view = document.getElementById('view');
  const id = view.querySelector('.id').value;
  const ext = view.querySelector('.file-ext').value;

  let url = ("/" + id);
  if (ext.length > 0) {
    url += "/" + ext;
  }
  console.log(url);
  window.open(url);
}

function modify() {
  // Essential info
  const modify = document.getElementById('modify');
  const id = modify.querySelector('.id').value;
  const key = modify.querySelector('.key').value;
  const card = modify.querySelector('.status-card');

  const h = new Headers();
  h.append('Key', key);

  // Update content
  const content = modify.querySelector('#modify-textarea').value;
  const files = modify.querySelector('.paste-from-file').files;
  const formData = new FormData();
  if (files.length > 0) {
    formData.append('content', files[0]);
  } else if (content.length > 0) {
    formData.append('content', content);
  }

  // Update name
  const name = modify.querySelector('.name').value;
  if (name.length > 0) {
    formData.appen('name', name);
  }

  // Update expire time
  const exp_time = modify.querySelector('.expire-in').value;
  const exp_time_unit = modify.querySelector('.expire-time-unit').value;
  if (exp_time.length != 0) {
    formData.append('expire_after', exp_time * exp_time_unit);
  }

  const req = new Request('/' + id, {mode: 'cors', method: 'PUT', headers: h, body: formData});

  // LINK START!
  update_card(card, working_card_class, "Working", "Hang tight...");
  fetch(req)
    .then(res => res.json())
    .then((res) => {
      if (res.success) {
        update_card(card, success_card_class, "Success", "Paste has been updated.");
      } else {
        update_card(card, fail_card_class, "Failed", res.message);
      }
    })
    .catch((err) => {
      update_card(card, fail_card_class, "Failed", err.message);
    });
}

function del() {
  // Essential info
  const main = document.getElementById('delete');
  const id = main.querySelector('.id').value;
  const key = main.querySelector('.key').value;
  const card = main.querySelector('.status-card');

  const h = new Headers();
  h.append('Key', key);

  const req = new Request('/' + id, {mode: 'cors', method: 'DELETE', headers: h});

  // LINK START!
  update_card(card, working_card_class, "Working", "Hang tight...");
  fetch(req)
    .then(res => res.json())
    .then((res) => {
      if (res.success) {
        update_card(card, success_card_class, "Success", "Paste has been deleted.");
        delete_paste_in_storage(id);
        update_paste_list();
      } else {
        update_card(card, fail_card_class, "Failed", res.message);
      }
    })
    .catch((err) => {
      update_card(card, fail_card_class, "Failed", err.message);
    });
}

