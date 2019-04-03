// "testgiver.addr" file>B 256 B>u@ 
0x538fa7cc24ff8eaa101d84a5f1ab7e832fe1d84b309cdfef4ee94373aac80f7d
dup constant wallet_addr ."Test giver address = " x. cr

$addr$

constant dest_addr

-1 constant wc
$seqno$ constant seqno

1000000000 constant Gram
{ Gram swap */ } : Gram*/

19.0 Gram*/ constant amount

// b x --> b'  ( serializes a Gram amount )
{ -1 { 1+ 2dup 8 * ufits } until
  rot over 4 u, -rot 8 * u, } : Gram, 
  
// create a message (NB: 01b00.., b = bounce)
<b b{010000100} s, wc 8 , dest_addr 256 u, amount Gram, 0 9 64 32 + + 1+ 1+ , "GIFT" $, b>
<b seqno 32 u, 1 8 u, swap ref, b>
dup ."enveloping message: " <s csr. cr
<b b{1000100} s, wc 8 , wallet_addr 256 u, 0 Gram, b{00} s,
   swap <s s, b>
dup ."resulting external message: " <s csr. cr
2 boc+>B dup Bx. cr
"wallet-query.boc" B>file
