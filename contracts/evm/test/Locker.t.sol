// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

import {Test, console} from "forge-std/Test.sol";
import {Token} from "../src/Token.sol";
import {Locker} from "../src/Locker.sol";

contract Locker_Test is Test {
    Locker public locker;
    Token public token;

    address payable developer = payable(makeAddr("developer"));
    address payable user = payable(makeAddr("user"));
    address payable canister = payable(makeAddr("canister"));
    uint256 defaultMinAmount = 100_000;

    function setUp() public {
        vm.broadcast(developer);
        token = new Token("USDC", 8, 10);
        vm.broadcast(developer);
        locker = new Locker(address(token), defaultMinAmount);
        assertEq(locker.getOwner(), developer);
        vm.broadcast(developer);
        locker.updateOwner(canister);
        vm.broadcast(developer);
        token.updateOwner(canister);
    }

    function testOnlyOwnerCanUpdateOwner() public {
        assertEq(locker.getOwner(), canister);
        vm.broadcast(developer);
        vm.expectRevert();
        locker.updateOwner(developer);
        vm.broadcast(canister);
        locker.updateOwner(developer);
        assertEq(locker.getOwner(), developer);
    }

    function testupdateMinAmount() public {
        assertEq(locker.getMinAmount(), defaultMinAmount);
        vm.broadcast(developer);
        vm.expectRevert();
        locker.updateMinAmount(0);
        vm.broadcast(canister);
        locker.updateMinAmount(0);
        assertEq(locker.getMinAmount(), 0);
    }

    function testLock1() public {
        vm.broadcast(canister);
        token.mint(user, 1_000_000);
        vm.broadcast(user);
        vm.expectRevert();
        locker.lock1(1_000_000, bytes32(uint256(123)));
        vm.broadcast(user);
        token.approve(address(locker), 10_000_000);
        vm.broadcast(user);
        vm.expectEmit();
        emit Locker.Lock1(user, 1_000_000, bytes32(uint256(123)));
        locker.lock1(1_000_000, bytes32(uint256(123)));
    }

    function testLock2() public {
        vm.broadcast(canister);
        token.mint(user, 1_000_000);
        vm.broadcast(user);
        token.approve(address(locker), 10_000_000);
        vm.broadcast(user);
        vm.expectEmit();
        emit Locker.Lock2(user, 1_000_000, bytes32(uint256(123)), bytes32(uint256(123)));
        locker.lock2(1_000_000, bytes32(uint256(123)), bytes32(uint256(123)));
    }

    function testLock3() public {
        vm.broadcast(canister);
        token.mint(user, 1_000_000);
        vm.broadcast(user);
        token.approve(address(locker), 10_000_000);
        vm.broadcast(user);
        vm.expectEmit();
        emit Locker.Lock3(user, 1_000_000, bytes32(uint256(123)), bytes32(uint256(123)), bytes32(uint256(123)));
        locker.lock3(1_000_000, bytes32(uint256(123)), bytes32(uint256(123)), bytes32(uint256(123)));
    }

    function testLock4() public {
        vm.broadcast(canister);
        token.mint(user, 1_000_000);
        vm.broadcast(user);
        token.approve(address(locker), 10_000_000);
        vm.broadcast(user);
        vm.expectEmit();
        emit Locker.Lock4(
            user, 1_000_000, bytes32(uint256(123)), bytes32(uint256(123)), bytes32(uint256(123)), bytes32(uint256(123))
        );
        locker.lock4(
            1_000_000, bytes32(uint256(123)), bytes32(uint256(123)), bytes32(uint256(123)), bytes32(uint256(123))
        );
    }

    function testOnlyOwnerCanTransferBatch() public {
        vm.broadcast(canister);
        token.mint(canister, 1_000_000);
        vm.broadcast(canister);
        token.approve(address(locker), 10_000_000);

        address payable[] memory recipients = new address payable[](1);
        uint256[] memory amounts = new uint256[](1);
        uint256[] memory ethAmounts = new uint256[](1);
        recipients[0] = user;
        amounts[0] = 100;

        vm.broadcast(user);
        vm.expectRevert();
        locker.batchTransfer(recipients, amounts, ethAmounts);

        vm.broadcast(canister);
        locker.batchTransfer(recipients, amounts, ethAmounts);
    }

    function testTransferBatch() public {
        vm.broadcast(canister);
        token.mint(canister, 1_000_000);
        vm.broadcast(canister);
        token.approve(address(locker), 10_000_000);

        address payable[] memory recipients = new address payable[](3);
        uint256[] memory amounts = new uint256[](3);
        uint256[] memory ethAmounts = new uint256[](0);

        recipients[0] = payable(makeAddr("user1"));
        amounts[0] = 100;
        recipients[1] = payable(makeAddr("user2"));
        amounts[1] = 100;
        recipients[2] = payable(makeAddr("user3"));
        amounts[2] = 100;

        vm.broadcast(canister);
        locker.batchTransfer(recipients, amounts, ethAmounts);

        assertEq(token.balanceOf(makeAddr("user1")), 100);
        assertEq(token.balanceOf(makeAddr("user2")), 100);
        assertEq(token.balanceOf(makeAddr("user3")), 100);
        assertEq(token.balanceOf(canister), 1_000_000 - 300);
    }

    function testTransferErc20AndEth() public {
        address payable recipient = payable(makeAddr("recipient"));
        uint256 tokenAmount = 100;
        uint256 ethAmount = 1 ether;

        vm.broadcast(canister);
        token.mint(canister, tokenAmount);
        vm.broadcast(canister);
        token.approve(address(locker), 10_000_000);

        // Fund the contract with ETH
        vm.deal(address(canister), ethAmount);

        uint256 initialCanisterTokenBalance = token.balanceOf(canister);
        uint256 initialCanisterEthBalance = canister.balance;
        uint256 initialRecipientEthBalance = recipient.balance;
        uint256 initialOwnerTokenBalance = token.balanceOf(recipient);

        vm.broadcast(canister);
        locker.transfer{value: ethAmount / 2}(recipient, tokenAmount);

        // Assert token balances
        assertEq(token.balanceOf(canister), initialCanisterTokenBalance - tokenAmount);
        assertEq(token.balanceOf(recipient), initialOwnerTokenBalance + tokenAmount);
        // Assert ETH balance
        assertEq(canister.balance, initialCanisterEthBalance - ethAmount / 2);
        assertEq(recipient.balance, initialRecipientEthBalance + ethAmount / 2);
    }

    function testTransferBatchWithEth() public {
        vm.broadcast(canister);
        token.mint(canister, 1_000_000);
        vm.broadcast(canister);
        token.approve(address(locker), 10_000_000);
        vm.deal(canister, 5 ether);
        assertEq(canister.balance, 5 ether);

        address payable[] memory recipients = new address payable[](3);
        uint256[] memory amounts = new uint256[](3);
        uint256[] memory ethAmounts = new uint256[](3);

        recipients[0] = payable(makeAddr("user1"));
        amounts[0] = 100;
        ethAmounts[0] = 1 ether;
        recipients[1] = payable(makeAddr("user2"));
        amounts[1] = 100;
        recipients[2] = payable(makeAddr("user3"));
        amounts[2] = 100;
        ethAmounts[2] = 1.5 ether;

        vm.broadcast(canister);
        locker.batchTransfer{value: 3 ether}(recipients, amounts, ethAmounts);

        assertEq(token.balanceOf(makeAddr("user1")), 100);
        assertEq(token.balanceOf(makeAddr("user2")), 100);
        assertEq(token.balanceOf(makeAddr("user3")), 100);
        assertEq(token.balanceOf(canister), 1_000_000 - 300);
        assertEq(recipients[0].balance, 1 ether);
        assertEq(recipients[1].balance, 0 ether);
        assertEq(recipients[2].balance, 1.5 ether);
        assertEq(address(locker).balance, 0.5 ether);
        assertEq(canister.balance, 2 ether);

        // Should not allow to withdraw more than the contract balance.
        vm.broadcast(canister);
        vm.expectRevert();
        locker.withdrawEth(0.6 ether, canister);

        // Only canister should be able to withdraw
        vm.broadcast(user);
        vm.expectRevert();
        locker.withdrawEth(0.5 ether, canister);

        vm.broadcast(canister);
        locker.withdrawEth(0.5 ether, canister);
        assertEq(address(locker).balance, 0 ether);
        assertEq(canister.balance, 2.5 ether);
    }
}
