// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

const STYLE: &str = r#"
#nav-bar {
  background-color: #333;
  overflow: hidden;
  margin-bottom: 20px;
  padding: 10px 0;
}

#nav-bar ul {
  list-style-type: none;
  margin: 0;
  padding: 0;
  display: flex;
  justify-content: center;
}

.tab {
  display: inline;
  padding: 14px 20px;
  cursor: pointer;
  color: white;
  text-align: center;
  text-decoration: none;
  background-color: #333;
  border: 1px solid #444;
  transition: background-color 0.3s ease;
}

.tab:hover {
  background-color: #575757;
}

.tab.active {
  background-color: #0077ff;
  border-color: #0055bb;
  font-weight: bold;
}

tbody tr:nth-child(odd) {
  background-color: #77bbcc;
}

tbody tr:nth-child(even) {
  background-color: #ee99ee;
}

"#;

const SCRIPT: &str = r#"
function showTab(index) {
  let tabs = document.querySelectorAll('[id^="tab-"]');
  let navItems = document.querySelectorAll('.tab');
  tabs.forEach(tab => tab.style.display = 'none');
  navItems.forEach(item => item.classList.remove('active'));
  document.getElementById('tab-' + index).style.display = 'block';
  navItems[index].classList.add('active');
}
"#;
