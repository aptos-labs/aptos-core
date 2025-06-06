
//////////////////////////////////////////////////////////////////////
// Auto‑generated dag graph  –  64 modules.
// Path not selected
//////////////////////////////////////////////////////////////////////

module 0xABCD::Y0 {
    use 0xABCD::Y1;
    use 0xABCD::Y2;
    use 0xABCD::Y3;
    use 0xABCD::Y4;
    use 0xABCD::Y5;
    use 0xABCD::Y6;
    use 0xABCD::Y7;
    use 0xABCD::Y8;
    use 0xABCD::Y9;
    use 0xABCD::Y10;
    use 0xABCD::Y11;
    use 0xABCD::Y12;
    use 0xABCD::Y13;
    use 0xABCD::Y14;
    use 0xABCD::Y15;
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public entry fun call(_account: &signer) {
        let _ = op();
    }

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y1::op();
        sum = sum + Y2::op();
        sum = sum + Y3::op();
        sum = sum + Y4::op();
        sum = sum + Y5::op();
        sum = sum + Y6::op();
        sum = sum + Y7::op();
        sum = sum + Y8::op();
        sum = sum + Y9::op();
        sum = sum + Y10::op();
        sum = sum + Y11::op();
        sum = sum + Y12::op();
        sum = sum + Y13::op();
        sum = sum + Y14::op();
        sum = sum + Y15::op();
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y1 {
    use 0xABCD::Y2;
    use 0xABCD::Y3;
    use 0xABCD::Y4;
    use 0xABCD::Y5;
    use 0xABCD::Y6;
    use 0xABCD::Y7;
    use 0xABCD::Y8;
    use 0xABCD::Y9;
    use 0xABCD::Y10;
    use 0xABCD::Y11;
    use 0xABCD::Y12;
    use 0xABCD::Y13;
    use 0xABCD::Y14;
    use 0xABCD::Y15;
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y2::op();
        sum = sum + Y3::op();
        sum = sum + Y4::op();
        sum = sum + Y5::op();
        sum = sum + Y6::op();
        sum = sum + Y7::op();
        sum = sum + Y8::op();
        sum = sum + Y9::op();
        sum = sum + Y10::op();
        sum = sum + Y11::op();
        sum = sum + Y12::op();
        sum = sum + Y13::op();
        sum = sum + Y14::op();
        sum = sum + Y15::op();
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y2 {
    use 0xABCD::Y3;
    use 0xABCD::Y4;
    use 0xABCD::Y5;
    use 0xABCD::Y6;
    use 0xABCD::Y7;
    use 0xABCD::Y8;
    use 0xABCD::Y9;
    use 0xABCD::Y10;
    use 0xABCD::Y11;
    use 0xABCD::Y12;
    use 0xABCD::Y13;
    use 0xABCD::Y14;
    use 0xABCD::Y15;
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y3::op();
        sum = sum + Y4::op();
        sum = sum + Y5::op();
        sum = sum + Y6::op();
        sum = sum + Y7::op();
        sum = sum + Y8::op();
        sum = sum + Y9::op();
        sum = sum + Y10::op();
        sum = sum + Y11::op();
        sum = sum + Y12::op();
        sum = sum + Y13::op();
        sum = sum + Y14::op();
        sum = sum + Y15::op();
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y3 {
    use 0xABCD::Y4;
    use 0xABCD::Y5;
    use 0xABCD::Y6;
    use 0xABCD::Y7;
    use 0xABCD::Y8;
    use 0xABCD::Y9;
    use 0xABCD::Y10;
    use 0xABCD::Y11;
    use 0xABCD::Y12;
    use 0xABCD::Y13;
    use 0xABCD::Y14;
    use 0xABCD::Y15;
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y4::op();
        sum = sum + Y5::op();
        sum = sum + Y6::op();
        sum = sum + Y7::op();
        sum = sum + Y8::op();
        sum = sum + Y9::op();
        sum = sum + Y10::op();
        sum = sum + Y11::op();
        sum = sum + Y12::op();
        sum = sum + Y13::op();
        sum = sum + Y14::op();
        sum = sum + Y15::op();
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y4 {
    use 0xABCD::Y5;
    use 0xABCD::Y6;
    use 0xABCD::Y7;
    use 0xABCD::Y8;
    use 0xABCD::Y9;
    use 0xABCD::Y10;
    use 0xABCD::Y11;
    use 0xABCD::Y12;
    use 0xABCD::Y13;
    use 0xABCD::Y14;
    use 0xABCD::Y15;
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y5::op();
        sum = sum + Y6::op();
        sum = sum + Y7::op();
        sum = sum + Y8::op();
        sum = sum + Y9::op();
        sum = sum + Y10::op();
        sum = sum + Y11::op();
        sum = sum + Y12::op();
        sum = sum + Y13::op();
        sum = sum + Y14::op();
        sum = sum + Y15::op();
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y5 {
    use 0xABCD::Y6;
    use 0xABCD::Y7;
    use 0xABCD::Y8;
    use 0xABCD::Y9;
    use 0xABCD::Y10;
    use 0xABCD::Y11;
    use 0xABCD::Y12;
    use 0xABCD::Y13;
    use 0xABCD::Y14;
    use 0xABCD::Y15;
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y6::op();
        sum = sum + Y7::op();
        sum = sum + Y8::op();
        sum = sum + Y9::op();
        sum = sum + Y10::op();
        sum = sum + Y11::op();
        sum = sum + Y12::op();
        sum = sum + Y13::op();
        sum = sum + Y14::op();
        sum = sum + Y15::op();
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y6 {
    use 0xABCD::Y7;
    use 0xABCD::Y8;
    use 0xABCD::Y9;
    use 0xABCD::Y10;
    use 0xABCD::Y11;
    use 0xABCD::Y12;
    use 0xABCD::Y13;
    use 0xABCD::Y14;
    use 0xABCD::Y15;
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y7::op();
        sum = sum + Y8::op();
        sum = sum + Y9::op();
        sum = sum + Y10::op();
        sum = sum + Y11::op();
        sum = sum + Y12::op();
        sum = sum + Y13::op();
        sum = sum + Y14::op();
        sum = sum + Y15::op();
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y7 {
    use 0xABCD::Y8;
    use 0xABCD::Y9;
    use 0xABCD::Y10;
    use 0xABCD::Y11;
    use 0xABCD::Y12;
    use 0xABCD::Y13;
    use 0xABCD::Y14;
    use 0xABCD::Y15;
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y8::op();
        sum = sum + Y9::op();
        sum = sum + Y10::op();
        sum = sum + Y11::op();
        sum = sum + Y12::op();
        sum = sum + Y13::op();
        sum = sum + Y14::op();
        sum = sum + Y15::op();
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y8 {
    use 0xABCD::Y9;
    use 0xABCD::Y10;
    use 0xABCD::Y11;
    use 0xABCD::Y12;
    use 0xABCD::Y13;
    use 0xABCD::Y14;
    use 0xABCD::Y15;
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y9::op();
        sum = sum + Y10::op();
        sum = sum + Y11::op();
        sum = sum + Y12::op();
        sum = sum + Y13::op();
        sum = sum + Y14::op();
        sum = sum + Y15::op();
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y9 {
    use 0xABCD::Y10;
    use 0xABCD::Y11;
    use 0xABCD::Y12;
    use 0xABCD::Y13;
    use 0xABCD::Y14;
    use 0xABCD::Y15;
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y10::op();
        sum = sum + Y11::op();
        sum = sum + Y12::op();
        sum = sum + Y13::op();
        sum = sum + Y14::op();
        sum = sum + Y15::op();
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y10 {
    use 0xABCD::Y11;
    use 0xABCD::Y12;
    use 0xABCD::Y13;
    use 0xABCD::Y14;
    use 0xABCD::Y15;
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y11::op();
        sum = sum + Y12::op();
        sum = sum + Y13::op();
        sum = sum + Y14::op();
        sum = sum + Y15::op();
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y11 {
    use 0xABCD::Y12;
    use 0xABCD::Y13;
    use 0xABCD::Y14;
    use 0xABCD::Y15;
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y12::op();
        sum = sum + Y13::op();
        sum = sum + Y14::op();
        sum = sum + Y15::op();
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y12 {
    use 0xABCD::Y13;
    use 0xABCD::Y14;
    use 0xABCD::Y15;
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y13::op();
        sum = sum + Y14::op();
        sum = sum + Y15::op();
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y13 {
    use 0xABCD::Y14;
    use 0xABCD::Y15;
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y14::op();
        sum = sum + Y15::op();
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y14 {
    use 0xABCD::Y15;
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y15::op();
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y15 {
    use 0xABCD::Y16;
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y16::op();
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y16 {
    use 0xABCD::Y17;
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y17::op();
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y17 {
    use 0xABCD::Y18;
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y18::op();
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y18 {
    use 0xABCD::Y19;
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y19::op();
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y19 {
    use 0xABCD::Y20;
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y20::op();
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y20 {
    use 0xABCD::Y21;
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y21::op();
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y21 {
    use 0xABCD::Y22;
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y22::op();
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y22 {
    use 0xABCD::Y23;
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y23::op();
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y23 {
    use 0xABCD::Y24;
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y24::op();
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y24 {
    use 0xABCD::Y25;
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y25::op();
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y25 {
    use 0xABCD::Y26;
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y26::op();
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y26 {
    use 0xABCD::Y27;
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y27::op();
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y27 {
    use 0xABCD::Y28;
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y28::op();
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y28 {
    use 0xABCD::Y29;
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y29::op();
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y29 {
    use 0xABCD::Y30;
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y30::op();
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y30 {
    use 0xABCD::Y31;
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y31::op();
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y31 {
    use 0xABCD::Y32;
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y32::op();
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y32 {
    use 0xABCD::Y33;
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y33::op();
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y33 {
    use 0xABCD::Y34;
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y34::op();
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y34 {
    use 0xABCD::Y35;
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y35::op();
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y35 {
    use 0xABCD::Y36;
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y36::op();
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y36 {
    use 0xABCD::Y37;
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y37::op();
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y37 {
    use 0xABCD::Y38;
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y38::op();
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y38 {
    use 0xABCD::Y39;
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y39::op();
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y39 {
    use 0xABCD::Y40;
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y40::op();
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y40 {
    use 0xABCD::Y41;
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y41::op();
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y41 {
    use 0xABCD::Y42;
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y42::op();
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y42 {
    use 0xABCD::Y43;
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y43::op();
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y43 {
    use 0xABCD::Y44;
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y44::op();
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y44 {
    use 0xABCD::Y45;
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y45::op();
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y45 {
    use 0xABCD::Y46;
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y46::op();
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y46 {
    use 0xABCD::Y47;
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y47::op();
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y47 {
    use 0xABCD::Y48;
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y48::op();
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y48 {
    use 0xABCD::Y49;
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y49::op();
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y49 {
    use 0xABCD::Y50;
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y50::op();
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y50 {
    use 0xABCD::Y51;
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y51::op();
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y51 {
    use 0xABCD::Y52;
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y52::op();
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y52 {
    use 0xABCD::Y53;
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y53::op();
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y53 {
    use 0xABCD::Y54;
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y54::op();
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y54 {
    use 0xABCD::Y55;
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y55::op();
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y55 {
    use 0xABCD::Y56;
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y56::op();
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y56 {
    use 0xABCD::Y57;
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y57::op();
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y57 {
    use 0xABCD::Y58;
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y58::op();
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y58 {
    use 0xABCD::Y59;
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y59::op();
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y59 {
    use 0xABCD::Y60;
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y60::op();
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y60 {
    use 0xABCD::Y61;
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y61::op();
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y61 {
    use 0xABCD::Y62;
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y62::op();
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y62 {
    use 0xABCD::Y63;

    public fun op(): u64 { 1 }

    public fun dummy(): u64 {
        let sum = 1u64;
        sum = sum + Y63::op();
        sum
    }
}

module 0xABCD::Y63 {
    public fun op(): u64 { 1 }
}
