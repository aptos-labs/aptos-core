import React from 'react'
import ReactDOM from 'react-dom'
import { BrowserRouter, Routes, Route } from 'react-router-dom'
import Wallet from './pages/Wallet'
import Login from './pages/Login'
import { GlobalStateProvider } from './GlobalState'

ReactDOM.render(
  <GlobalStateProvider>
    <BrowserRouter>
      <Routes>
        <Route path='/' element={<Login/>}></Route>
        <Route path='/wallet' element={<Wallet/>}></Route>
      </Routes>
    </BrowserRouter>
  </GlobalStateProvider>,
  document.getElementById('root')
)
